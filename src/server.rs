use std::cmp::min;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};

use crossbeam::channel;
use num_cpus;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tokio::sync::Notify;

use crate::server_utils::get_task_value;
use crate::task::{Task, TaskType};

// Limit to number of CPU bound tasks specified in requirements
static CPU_BOUND_TASKS_COUNT: AtomicUsize = AtomicUsize::new(0);
const MAX_CPU_BOUND_TASKS: usize = 40;
static NOTIFY: OnceCell<Arc<Notify>> = OnceCell::new();


pub trait ServerTrait {
    fn start_server(
        &self,
        address: String,
        tx: mpsc::Sender<Result<(), Box<dyn Error + Send>>>,
    );
}

pub struct Server;

impl ServerTrait for Server {
    fn start_server(&self, address: String, tx: mpsc::Sender<Result<(), Box<dyn Error + Send>>>) {
        println!("Starting the server");

        let listener = TcpListener::bind(address);

        match listener {
            Ok(_) => tx.send(Ok(())).unwrap(),
            Err(e) => {
                println!("here {}", e);
                tx.send(Err(Box::new(e))).unwrap();
                return;
            }
        };

        // Set up thread pool
        let num_cpus = num_cpus::get(); // Retrieve number of CPUs
        let num_threads = min(2 * num_cpus, 20);
        eprintln!("Found {} cpus. Setting up thread pool with {} threads.", num_cpus, num_threads);

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();

        // Create an MPMC channel for distributing connections to worker threads
        let (conn_tx, conn_rx) = channel::unbounded::<TcpStream>();

        // Spawn worker threads
        for _ in 0..num_cpus {
            let conn_tx = conn_tx.clone();
            let conn_rx = conn_rx.clone();
            pool.spawn(move || {
                // Create a Tokio runtime for each worker thread
                let runtime = Runtime::new().unwrap();

                // Handle connections received from the channel
                while let Ok(stream) = conn_rx.recv() {
                    let stream = stream;
                    runtime.spawn(async move {
                        Self::handle_connection(stream).await;
                    });
                }

                // Drop the cloned Sender when the worker thread exits
                drop(conn_tx);
            });
        }

        // Accept incoming connections and distribute them to worker threads
        for stream in listener.unwrap().incoming() {
            match stream {
                Ok(stream) => {
                    conn_tx.send(stream).unwrap();
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

impl Server {
    async fn handle_connection(mut stream: TcpStream) {
        loop {
            let mut buf_reader = BufReader::new(&mut stream);
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    return;
                }
                Ok(_) => {
                    let (task_type, seed) = get_task_value(line).unwrap();
                    let task_type = TaskType::from_u8(task_type).unwrap();
                    let notify = NOTIFY.get_or_init(|| Arc::new(Notify::new()));
                    match task_type {
                        TaskType::CpuIntensiveTask => {
                            loop {
                                let mut current_count = CPU_BOUND_TASKS_COUNT.load(Ordering::SeqCst);
                                if current_count > MAX_CPU_BOUND_TASKS {
                                    // Hit limit on CPU bound tasks
                                    notify.notified().await;
                                    continue;
                                }
                                let incremented = CPU_BOUND_TASKS_COUNT.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                                    // Atomically increase count of CPU bound tasks
                                    if x < MAX_CPU_BOUND_TASKS { Some(x + 1) } else { None }
                                });

                                if incremented.is_ok() {
                                    break; // Successfully incremented counter, can proceed.
                                }

                                // The increment failed because another thread beat us to it; wait for a notification before trying again.
                                notify.notified().await;
                            }
                            let r = Task::execute_async(task_type as u8, seed).await;
                            CPU_BOUND_TASKS_COUNT.fetch_sub(1, Ordering::SeqCst);
                            // Notify all awaiting tasks that there may be available capacity.
                            notify.notify_waiters();
                            stream.write(&[r]).unwrap();
                        }
                        _ => {
                            let r = Task::execute_async(task_type as u8, seed).await;
                            stream.write(&[r]).unwrap();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Unable to get command due to: {}", e);
                    return;
                }
            }
        }
    }
}
