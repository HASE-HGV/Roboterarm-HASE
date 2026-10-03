use crate::config::{POSITION_FORMAT, RAW_FORMAT};

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
        "{}\n{}\n\n{}\n  --cli    Prompt for one XYZ position and execute it.\n  --shell  Repeatedly read position commands from an interactive input loop.\n  --raw    Repeatedly read raw angles from an interactive input loop.\n  --site   Serve the control page and JSON API on the local network.\n  --help   Show this guide.\n\n{}\n  {POSITION_FORMAT}\n\n{}\n  {RAW_FORMAT}\n\n{}\n  The trailing start angles are optional. Leave them at 0 when the axes are\n  homed; set them to the angles the arm is already at to command a relative move.\n\n{}\n  cargo run -- --cli\n  cargo run -- --shell\n  cargo test\n\n{}\n  RUSTCTL_SITE_ADDR=0.0.0.0:8080 cargo build --release --features hardware\n  sudo ./target/release/rustctl --site",
        header("Roboterarm controller"),
        info(&format!(
            "Usage: {program} --cli | --shell | --raw | --site | --help"
        )),
        header("Modes"),
        header("Position command format"),
        header("Raw angle command format"),
        header("Start position"),
        header("PC testing"),
        header("Raspberry Pi hardware"),
    )
}
