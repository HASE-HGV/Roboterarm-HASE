use std::net::IpAddr;

use crate::utils::AppError;

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

