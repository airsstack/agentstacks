use std::io::{self, BufRead, IsTerminal};

fn main() {
    let stdin = io::stdin();
    println!("stdin.is_terminal() = {}", stdin.is_terminal());
    println!("stdout.is_terminal() = {}", io::stdout().is_terminal());
    let mut line = String::new();
    let r = stdin.lock().read_line(&mut line);
    println!("read_line -> {r:?}, line = {line:?}");
}
