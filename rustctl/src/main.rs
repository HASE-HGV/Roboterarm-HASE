mod api;
mod bresenham;
mod cli;
mod config;
mod control;
mod cv;
mod error;
mod http_api;
mod kinematics;
mod motion;
mod net;
mod pretty;
mod shell;

use std::env;

fn welcome() {
    println!();
    println!();
    println!();
    println!(r#"                               Welcome to:           "#);
    println!(r#"                                                     "#);
    println!(r#"                                      _       _   _  "#);
    println!(r#"                                     | |     | | | | "#);
    println!(r#"                       _ __ _   _ ___| |_ ___| |_| | "#);
    println!(r#"                      | '__| | | / __| __/ __| __| | "#);
    println!(r#"                      | |  | |_| \__ \ || (__| |_| | "#);
    println!(r#"                      |_|   \__,_|___/\__\___|\__|_| "#);
    println!();
    println!();
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    welcome();
    let args: Vec<String> = env::args().collect();
    let config_arg = args
        .iter()
        .position(|arg| arg == "--config")
        .map(|index| {
            args.get(index + 1)
                .cloned()
                .ok_or("--config requires a file name or path")
        })
        .transpose()?;
    config::load_settings(config_arg.as_deref())?;
    let config = config::settings();
    println!(
        "{}",
        // printing the current config pulled from rustctl/src/config/configs.toml
        pretty::info(&format!(
            "gear_ratio={:.2} | target={}µs (2 × pulse={}µs + overhead={}µs) | GPIO M1={:?} M2={:?} M3={:?} M4={:?}",
            config.hardware.gear_ratio,
            config.hardware.target_time_us(),
            config.hardware.pulse_t_us,
            config.hardware.overhead_us,
            config.pins.m1,
            config.pins.m2,
            config.pins.m3,
            config.pins.m4
        ))
    );
    let mode_args: Vec<String> = args
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            *index == 0
                || (args.get(index - 1).is_none_or(|arg| arg != "--config")
                    && args[*index] != "--config")
        })
        .map(|(_, arg)| arg.clone())
        .collect();
    cli::get_mode(&mode_args)
}

#[cfg(test)]
#[allow(dead_code, unused_imports)]
mod coverage_tests {
    use super::*;
    use crate::api::*;
    use crate::bresenham::*;
    use crate::config::*;
    use crate::kinematics::*;

    include!("../tests/unit/support.rs");

    mod api_tests {
        use super::*;
        include!("../tests/unit/api.rs");
    }

    mod cli_shell_tests {
        use super::*;
        include!("../tests/unit/cli_shell.rs");
    }

    mod config_tests {
        use super::*;
        include!("../tests/unit/config.rs");
    }

    mod control_tests {
        use super::*;
        include!("../tests/unit/control.rs");
    }

    mod http_tests {
        use super::*;
        include!("../tests/unit/http.rs");
    }

    mod server_tests {
        use super::*;
        include!("../tests/unit/http_server.rs");
    }

    mod kinematics_tests {
        use super::*;
        include!("../tests/unit/kinematics.rs");
    }

    mod motion_tests {
        use super::*;
        include!("../tests/unit/motion.rs");
    }

    mod network_tests {
        use super::*;
        include!("../tests/unit/network.rs");
    }
}
