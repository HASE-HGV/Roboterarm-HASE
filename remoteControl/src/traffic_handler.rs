pub(crate) fn init_connection(ip: &str, mode: u8) {
    let mode_parsed = match mode {
        1 => "raw",
        2 => "inverse Kinematics (ik)",
        _ => "Unknown Status", // Match arms must be exhaustive!
    };

    println!("ip: {ip}, mode: {mode_parsed}")
}