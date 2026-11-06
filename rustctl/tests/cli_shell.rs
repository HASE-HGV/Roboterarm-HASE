#![allow(dead_code, unused_imports)]

include!("unit/integration_setup.rs");

mod tests {
    use super::*;

    include!("unit/support.rs");

    mod cli_shell_tests {
        include!("unit/cli_shell.rs");
    }
}
