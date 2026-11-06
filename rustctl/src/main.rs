mod api;
mod bresenham;
mod cli;
mod config;
mod control;
mod http_api;
mod kinematics;
mod motion;
mod net;
mod pretty;
mod shell;

use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    cli::get_mode(&args)
}
