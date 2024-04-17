use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;

use num_cpus;

use crate::server_utils::get_task_value;
use crate::task::Task;

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
        eprintln!("Using all {} cpus found to set up a thread pool.", num_cpus);

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus)
            .build()
            .unwrap();

        for stream in listener.unwrap().incoming() {
            match stream {
                Ok(stream) => {
                    let stream = stream;
                    pool.spawn(move || {
                        Self::handle_connection(stream);
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

impl Server {
    fn handle_connection(mut stream: TcpStream) {
        loop {
            let mut buf_reader = BufReader::new(&mut stream);
            let mut line = String::new();
            match buf_reader.read_line(&mut line) {
                Ok(0) => {
                    return;
                }
                Ok(_) => {
                    let response = get_task_value(line);
                    if let Some(r) = response {
                        eprintln!("Finished executing task with new seed {}", r);
                        stream.write(&[r]).unwrap();
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
