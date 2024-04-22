pub fn get_task_value(buf: String) -> Result<(u8, u64), Box<dyn std::error::Error>> {
    let parse_input = || -> Result<(u8, u64), Box<dyn std::error::Error>> {
        let parts: Vec<&str> = buf.trim().split(':').collect();
        let task_type = parts.first().unwrap().parse::<u8>()?;
        let seed = parts.last().unwrap().parse::<u64>()?;
        Ok((task_type, seed))
    };

    parse_input()

    /*
    match parse_input() {
        Ok((task_type, seed)) => Some(Task::execute(task_type, seed)),
        Err(_) => None
    }
     */
}
