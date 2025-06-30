use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

mod logging;
mod core;
mod netstat;
use crate::core::{decide_action, Action};

fn bind_mount(path: &str, rw: bool) -> [String; 3] {
    let flag = if rw { "--bind" } else { "--ro-bind" };
    [flag.to_string(), path.to_string(), path.to_string()]
}

fn to_bubblewrap_args(args: &[String]) -> Vec<String> {
    let mut bwrap_args: Vec<String> = [
        "--die-with-parent", "--unshare-pid", "--new-session",
        "--proc", "/proc", "--dev", "/dev",
        "--symlink", "usr/lib64", "/lib64",
    ]
    .iter().map(String::from).collect();

    bwrap_args.extend(["/bin", "/usr", "/lib"].iter().flat_map(|&path| bind_mount(path, false)));

    if args.iter().any(|arg| arg == "--allow-net") {
        bwrap_args.push("--share-net".to_string());
        bwrap_args.extend(["/etc/resolv.conf", "/etc/ssl"].iter().flat_map(|&path| bind_mount(path, false)));
    }

    let read_args = args.iter()
        .filter_map(|arg| arg.strip_prefix("--allow-read="))
        .flat_map(|paths| paths.split(','))
        .filter(|path| !path.is_empty())
        .flat_map(|path| bind_mount(path, false));

    let write_args = args.iter()
        .filter_map(|arg| arg.strip_prefix("--allow-write="))
        .flat_map(|paths| paths.split(','))
        .filter(|path| !path.is_empty())
        .flat_map(|path| bind_mount(path, true));

    bwrap_args.extend(read_args);
    bwrap_args.extend(write_args);

    bwrap_args
}

fn spawn_and_wait_for_port(command: &mut Command, port: u16) {
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to spawn child process: {}", e);
            std::process::exit(1);
        }
    };

    let start = Instant::now();
    let timeout = Duration::from_secs(30);

    loop {
        if netstat::is_port_listening(port) {
            eprintln!("READY");
            break;
        }

        if start.elapsed() > timeout {
            eprintln!("error: timed out waiting for port {}", port);
            let _ = child.kill();
            std::process::exit(1);
        }

        thread::sleep(Duration::from_millis(100));
    }

    let status = child.wait().expect("Failed to wait on child process");
    if let Some(code) = status.code() {
        std::process::exit(code);
    } else {
        // Process terminated by signal
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let cwd = env::current_dir().map_or_else(|_| "unknown".to_string(), |p| p.display().to_string());
    debug_log!("CWD: {}", cwd);

    let quoted_args: Vec<String> = args
        .iter()
        .map(|arg| {
            if arg.is_empty() {
                "''".to_string()
            } else if arg
                .chars()
                .all(|c| c.is_alphanumeric() || "/_.-".contains(c))
            {
                arg.clone()
            } else {
                format!("'{}'", arg.replace('\'', "'\\''"))
            }
        })
        .collect();
    debug_log!("{}", quoted_args.join(" "));

    let mut path_var = env::var("PATH").unwrap_or_default();
    if path_var.is_empty() {
        if let Ok(output) = Command::new("/bin/bash")
            .arg("--login")
            .arg("-c")
            .arg("echo $PATH")
            .output()
        {
            if output.status.success() {
                path_var = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
    }

    match decide_action(&args, &path_var) {
        Action::Exec(config, deno_args) => {
            let bwrap_args = to_bubblewrap_args(&args);
            let mut command = Command::new("bwrap");
            command.args(&bwrap_args);
            command.arg("--");
            command.arg("/bin/sh");
            command.env("PORT", deno_args.port.to_string());
            let shell_script = format!("set -x\n{}", &config.exec);
            fs::write("cmd.sh", &shell_script).expect("Unable to write to cmd.sh");
            command.arg("cmd.sh");
            debug_log!("Spawning command: {:?}", &command);
            spawn_and_wait_for_port(&mut command, deno_args.port);
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
    use crate::core::{decide_action, Action, DenoArgs};
    use std::env;
    use std::path::Path;
    use tempfile::tempdir;

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
            json_arg.clone(),
        ];

        let path_var = "/usr/bin:/bin";
        let dir = file_path.parent().unwrap().to_str().unwrap();
        let args_str = format!("deno run --allow-net '{}'", json_arg);
        println!(
            "Equivalent command for test_invoke_adapter:\ncd {} && {}",
            dir, args_str
        );

        let action = decide_action(&args, path_var);
        let expected_deno_args = DenoArgs {
            command: "fetch".to_string(),
            entrypoint,
            port: 38025,
        };

        match action {
            Action::Exec(config, deno_args) => {
                assert!(config.exec.contains("$PORT"));
                assert_eq!(deno_args, expected_deno_args);
            }
            _ => panic!("Expected Action::Exec, but got {:?}", action),
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
            json_arg.clone(),
        ];

        let temp_dir = tempdir().unwrap();
        let deno_dir = temp_dir.path();
        std::fs::File::create(deno_dir.join("deno")).unwrap();

        let original_path =
            env::join_paths([deno_dir, Path::new("/usr/bin"), Path::new("/bin")].iter()).unwrap();

        let action = decide_action(&args, original_path.to_str().unwrap());

        let expected_new_path =
            env::join_paths([Path::new("/usr/bin"), Path::new("/bin")].iter()).unwrap();

        assert_eq!(
            action,
            Action::ExecDeno {
                new_path: Some(expected_new_path)
            }
        );
    }

    #[test]
    fn test_jsr_entrypoint() {
        let entrypoint = "jsr:@smallweb/file-server@0.8.2".to_string();
        let json_arg = format!(
            r#"{{"command":"fetch","entrypoint":"{}","port":42541}}"#,
            entrypoint
        );
        let args = vec![
            "/path/to/adapter/deno".to_string(),
            "run".to_string(),
            "-".to_string(),
            json_arg.clone(),
        ];

        let temp_dir = tempdir().unwrap();
        let deno_dir = temp_dir.path();
        std::fs::File::create(deno_dir.join("deno")).unwrap();

        let original_path =
            env::join_paths([deno_dir, Path::new("/usr/bin"), Path::new("/bin")].iter()).unwrap();

        let action = decide_action(&args, original_path.to_str().unwrap());

        let expected_new_path =
            env::join_paths([Path::new("/usr/bin"), Path::new("/bin")].iter()).unwrap();

        assert_eq!(
            action,
            Action::ExecDeno {
                new_path: Some(expected_new_path)
            }
        );
    }
}
