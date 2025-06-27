use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::Command;

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // command and port are unused for now
pub struct DenoArgs {
    command: String,
    entrypoint: String,
    port: u16,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields are not used, we only care about parsing success.
pub struct MainTsxConfig {
    watchpattern: String,
    exec: String,
    build: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Print(String),
    ExecDeno { new_path: Option<OsString> },
}

pub fn decide_action(args: &[String], path_var: &str) -> Action {
    let should_change_path = args.get(0).map_or(false, |a| a.ends_with("deno"));

    let create_new_path = || {
        if should_change_path {
            let mut paths: Vec<_> = env::split_paths(path_var).collect();
            if !paths.is_empty() {
                paths.remove(0);
            }
            Some(env::join_paths(paths).unwrap())
        } else {
            None
        }
    };

    let fallback = || Action::ExecDeno {
        new_path: create_new_path(),
    };

    let last_arg = if let Some(arg) = args.last() {
        arg
    } else {
        return fallback();
    };

    let deno_args = if let Ok(args) = serde_json::from_str::<DenoArgs>(last_arg) {
        args
    } else {
        return fallback();
    };

    let path_str = if let Some(p) = deno_args.entrypoint.strip_prefix("file://") {
        p
    } else {
        return fallback();
    };

    if !path_str.ends_with("main.tsx") {
        return fallback();
    }

    let file_content = if let Ok(content) = fs::read_to_string(path_str) {
        content
    } else {
        return fallback();
    };

    if !file_content.starts_with('{') {
        return fallback();
    }

    if serde_json::from_str::<MainTsxConfig>(&file_content).is_ok() {
        Action::Print(file_content)
    } else {
        fallback()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_invoke_adapter() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.tsx");
        let mut file = File::create(&file_path).unwrap();
        let config_content = r#"{"watchpattern": "src/**/*.rs", "exec": "cargo run"}"#;
        writeln!(file, "{}", config_content).unwrap();

        let entrypoint = format!("file://{}", file_path.to_str().unwrap());
        let json_arg = format!(
            r#"{{"command":"fetch","entrypoint":"{}","port":38025}}"#,
            entrypoint
        );
        let args = vec![
            "deno".to_string(),
            "run".to_string(),
            "--allow-net".to_string(),
            json_arg,
        ];

        let action = decide_action(&args, "/usr/bin:/bin");
        // read_to_string reads the newline from writeln!
        assert_eq!(action, Action::Print(format!("{}\n", config_content)));
    }

    #[test]
    fn test_normal_deno() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.tsx");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "console.log('Hello, world!');").unwrap();

        let entrypoint = format!("file://{}", file_path.to_str().unwrap());
        let json_arg = format!(
            r#"{{"command":"fetch","entrypoint":"{}","port":38025}}"#,
            entrypoint
        );
        let args = vec![
            "/path/to/adapter/deno".to_string(),
            "run".to_string(),
            "--allow-net".to_string(),
            json_arg,
        ];

        let original_path = "/path/to/adapter:/usr/bin:/bin";
        let action = decide_action(&args, original_path);

        let mut paths: Vec<_> = env::split_paths(original_path).collect();
        paths.remove(0);
        let expected_new_path = env::join_paths(paths).unwrap();

        assert_eq!(
            action,
            Action::ExecDeno {
                new_path: Some(expected_new_path)
            }
        );
    }
}
