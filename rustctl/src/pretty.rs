pub(crate) fn title(text: &str) -> String {
    format!("\n== {text} ==")
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
        "Roboterarm controller\n\nUsage: {program} --cli | --shell | --raw | --api | --help\n\nModes:\n  --cli   Prompt for one XYZ position and execute it.\n  --shell Repeatedly read position commands from an interactive input loop.\n  --raw   Repeatedly read raw angles from an interactive input loop.\n  --api   Run the HTTP API server.\n  --help  Show this guide.\n\nPosition command format:\n  radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive\nTiming is fixed: total period = 1083 us, pulse width = 500 us.\n\nPC testing:\n  cargo run -- --cli\n  cargo run -- --shell\n  cargo test\n\nRaspberry Pi hardware:\n  cargo build --release --features hardware\n  sudo ./target/release/rustctl --shell"
    )
}
