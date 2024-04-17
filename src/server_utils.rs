use crate::task::{Task, TaskType};

pub fn get_task_value(buf: String) -> Option<u8> {
    let parse_input = || -> Result<(u8, u64), Box<dyn std::error::Error>> {
        let parts: Vec<&str> = buf.trim().split(':').collect();
        let task_type = parts.first().unwrap().parse::<u8>()?;
        let seed = parts.last().unwrap().parse::<u64>()?;
        eprintln!("Received {:?} with seed {}", TaskType::from_u8(task_type).unwrap(), seed);
        Ok((task_type, seed))
    };

    match parse_input() {
        Ok((task_type, seed)) => Some(Task::execute(task_type, seed)),
        Err(_) => None
    }
}
