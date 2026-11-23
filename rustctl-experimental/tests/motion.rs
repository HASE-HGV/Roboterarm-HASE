#![allow(dead_code, unused_imports)]

include!("unit/integration_setup.rs");

mod tests {
    use super::*;

    include!("unit/support.rs");

    mod motion_tests {
        include!("unit/motion.rs");
    }
}
