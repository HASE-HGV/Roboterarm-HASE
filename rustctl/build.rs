use std::fs;
use std::path::Path;

fn count_tests(directory: &Path) -> usize {
    fs::read_dir(directory)
        .expect("read tests directory")
        .map(|entry| {
            let path = entry.expect("read tests directory entry").path();
            if path.is_dir() {
                count_tests(&path)
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                fs::read_to_string(path)
                    .expect("read Rust test source")
                    .matches("#[test]")
                    .count()
            } else {
                0
            }
        })
        .sum()
}

fn main() {
    println!("cargo:rerun-if-changed=tests");
    println!(
        "cargo:rustc-env=RUSTCTL_TOTAL_TESTS={}",
        count_tests(Path::new("tests"))
    );
}
