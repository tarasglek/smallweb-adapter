use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
struct DenoArgs {
    command: String,
    entrypoint: String,
    port: u16,
}

fn main() {
    let last_arg = match env::args().last() {
        Some(arg) => arg,
        None => {
            // This case is unlikely as args[0] is the program name.
            eprintln!("No command line arguments found.");
            std::process::exit(1);
        }
    };

    match serde_json::from_str::<DenoArgs>(&last_arg) {
        Ok(deno_args) => {
            println!("Parsed args: {:?}", deno_args);
        }
        Err(e) => {
            eprintln!("Failed to parse JSON from last argument: {}", e);
            std::process::exit(1);
        }
    }
}
