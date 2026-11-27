use std::io::stdin;

pub(crate) fn input() -> String {
    let mut s = String::new();

    stdin()
        .read_line(&mut s)
        .expect("not a valid &str");
    
    if let Some('\n') = s.chars().next_back() {
        s.pop();
    }
    if let Some('\r') = s.chars().next_back() {
        s.pop();
    }
    
    s
}