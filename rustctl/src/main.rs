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
