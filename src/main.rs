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

#[cfg(test)]
mod tests {
    use crate::core::{decide_action, Action};
    use std::env;

    #[test]
    fn test_invoke_adapter() {
        let file_path = std::path::Path::new("test/invoke_adapter/main.tsx");
        let absolute_path = std::fs::canonicalize(file_path).unwrap();
        let entrypoint = format!("file://{}", absolute_path.to_str().unwrap());
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

        let path_var = "/usr/bin:/bin";
        let dir = file_path.parent().unwrap().to_str().unwrap();
        let args_str = format!("deno run --allow-net '{}'", json_arg);
        println!(
            "Equivalent command for test_invoke_adapter:\ncd {} && {}",
            dir, args_str
        );

        let action = decide_action(&args, path_var);
        match action {
            Action::Print(content) => {
                assert!(content.contains(r#""exec":"#));
            }
            _ => panic!("Expected Action::Print, but got {:?}", action),
        }
    }

    #[test]
    fn test_normal_deno() {
        let file_path = std::path::Path::new("test/normal_deno/main.tsx");
        let absolute_path = std::fs::canonicalize(file_path).unwrap();
        let entrypoint = format!("file://{}", absolute_path.to_str().unwrap());
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
        let dir = file_path.parent().unwrap().to_str().unwrap();
        let args_str = format!("{} run --allow-net '{}'", args[0], json_arg);

        println!(
            "Equivalent command for test_normal_deno:\ncd {} && {}",
            dir, args_str
        );

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
