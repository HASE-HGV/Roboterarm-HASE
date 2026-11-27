use std::net::IpAddr;

use crate::utils::{AppError, Data};

pub(crate) fn configure_connection(ip: &str, mode: u8) -> Result<(), AppError> {
    let ip: IpAddr = ip.trim().parse()?;
    let mode_name = match mode {
        1 => "raw",
        2 => "inverse Kinematics (ik)",
        _ => return Err(AppError::InvalidRange),
    };

    println!("ip: {ip}, mode: {mode_name}");
    Ok(())
}

pub(crate) fn connect(data: &Data) -> Result<(), AppError> {
    println!("connect (placeholder): ip: {}, mode: {}", data.ip, data.mode);
    Ok(())
}

pub(crate) fn run_get() -> Result<(), AppError>{
    println!("Please enter the type of information you wish to receive");
    Ok(())
}

pub(crate) fn run_post() -> Result<(), AppError>{
    println!("Please enter the type of information you wish to send");
    Ok(())
}