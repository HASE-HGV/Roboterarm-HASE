mod api;
mod bresenham;
mod cli;
mod config;
mod http_api;
mod kinematics;
mod motion;
mod pretty;
mod shell;

use std::env;

#[cfg(test)]
pub(crate) use api::*;
#[cfg(test)]
pub(crate) use bresenham::*;
#[cfg(test)]
pub(crate) use config::*;
#[cfg(test)]
pub(crate) use kinematics::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    cli::get_mode(&args)
}

#[cfg(test)]
mod tests {
    use super::*;

    include!("tests.rs");
}
