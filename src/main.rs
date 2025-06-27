use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;

mod core;
use crate::core::{decide_action, Action};

fn main() {
    let args: Vec<String> = env::args().collect();
    let path_var = env::var("PATH").unwrap_or_default();

    match decide_action(&args, &path_var) {
        Action::Print(config) => {
            println!("{}", config);
        }
        Action::ExecDeno { new_path } => {
            let mut command = Command::new("deno");
            command.args(&args[1..]);
            if let Some(p) = new_path {
                command.env("PATH", p);
            }
            let err = command.exec();
            eprintln!("Failed to exec deno: {}", err);
            std::process::exit(1);
        }
    }
}
