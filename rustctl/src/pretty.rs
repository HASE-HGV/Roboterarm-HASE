pub(crate) fn header(text: &str) -> String {
    format!("== {text} ==")
}

pub(crate) fn title(text: &str) -> String {
    format!("\n{}", header(text))
}

pub(crate) fn info(text: &str) -> String {
    format!("[>] {text}")
}

pub(crate) fn success(text: &str) -> String {
    format!("[OK] {text}")
}

pub(crate) fn warning(text: &str) -> String {
    format!("[!] {text}")
}

pub(crate) fn prompt(label: &str, example: &str) -> String {
    format!("{label} [{example}]: ")
}

pub(crate) fn help(program: &str) -> String {
    format!(
        "{}\n{}\n\n{}\n  --cli   Prompt for one XYZ position and execute it.\n  --shell Repeatedly read position commands from an interactive input loop.\n  --raw   Repeatedly read raw angles from an interactive input loop.\n  --api   Run the HTTP API server.\n  --help  Show this guide.\n\n{}\n  radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive\n{}\n\n{}\n  cargo run -- --cli\n  cargo run -- --shell\n  cargo test\n\n{}\n  cargo build --release --features hardware\n  sudo ./target/release/rustctl --shell",
        header("Roboterarm controller"),
        info(&format!(
            "Usage: {program} --cli | --shell | --raw | --api | --help"
        )),
        header("Modes"),
        header("Position command format"),
        info("Timing is fixed: total period = 1083 us, pulse width = 500 us."),
        header("PC testing"),
        header("Raspberry Pi hardware")
    )
}
