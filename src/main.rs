use serde::Deserialize;
use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::Command;

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // command and port are unused for now
struct DenoArgs {
    command: String,
    entrypoint: String,
    port: u16,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields are not used, we only care about parsing success.
struct MainTsxConfig {
    watchpattern: String,
    exec: String,
    build: Option<String>,
}

fn launch_deno() {
    let path_var = match env::var("PATH") {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Could not get PATH environment variable: {}", e);
            std::process::exit(1);
        }
    };

    let mut paths: Vec<_> = env::split_paths(&path_var).collect();
    if !paths.is_empty() {
        paths.remove(0);
    }

    let new_path = match env::join_paths(paths) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Could not construct new PATH: {}", e);
            std::process::exit(1);
        }
    };

    let args: Vec<String> = env::args().collect();
    let mut command = Command::new("deno");
    command.args(&args[1..]);
    command.env("PATH", new_path);

    let err = command.exec();
    eprintln!("Failed to exec deno: {}", err);
    std::process::exit(1);
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
            let path_str = if let Some(p) = deno_args.entrypoint.strip_prefix("file://") {
                p
            } else {
                launch_deno();
                return;
            };

            if !path_str.ends_with("main.tsx") {
                launch_deno();
                return;
            }

            let file_content = match fs::read_to_string(path_str) {
                Ok(content) => content,
                Err(_) => {
                    launch_deno();
                    return;
                }
            };

            if !file_content.starts_with('{') {
                launch_deno();
                return;
            }

            match serde_json::from_str::<MainTsxConfig>(&file_content) {
                Ok(_config) => {
                    // Per README, if parse succeeds, print the json.
                    println!("{}", file_content);
                }
                Err(_) => {
                    // Fails to parse as JSON
                    launch_deno();
                }
            }
        }
        Err(_) => {
            // Not a smallweb command, probably a user running deno directly.
            launch_deno();
        }
    }
}
