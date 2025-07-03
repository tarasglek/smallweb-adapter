use std::env;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[macro_use]
mod logging;
mod core;
mod linux;
use crate::core::{decide_action, Action};

fn check_child_status(child: &mut std::process::Child) {
    match child.try_wait() {
        Ok(Some(status)) => {
            eprintln!("error: child process exited early with status: {}", status);
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(None) => { /* child is still running */ }
        Err(e) => {
            eprintln!("error: failed to check child process status: {}", e);
            let _ = child.kill();
            std::process::exit(1);
        }
    }
}

fn spawn_and_wait_for_port(command: &mut Command, port: u16, shell_script: Option<&str>) {
    if shell_script.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to spawn child process: {}", e);
            std::process::exit(1);
        }
    };

    if let Some(script) = shell_script {
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(script.as_bytes()) {
                eprintln!("error: failed to write to child stdin: {}", e);
                let _ = child.kill();
                std::process::exit(1);
            }
        }
    }

    let start = Instant::now();
    let timeout = Duration::from_secs(30);

    loop {
        check_child_status(&mut child);

        if linux::is_port_listening(port) {
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
    std::process::exit(status.code().unwrap_or(1));
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|arg| arg == "--help") {
        println!("smallweb-adapter: a not-deno adapter for smallweb.");
        println!("For more information, visit https://github.com/tarasglek/smallweb-adapter");
        std::process::exit(0);
    }

    if args.iter().any(|arg| arg == "--smallweb-adapter-version") {
        println!("smallweb-adapter v{}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

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

	// workaround for smallweb not passing PATH through
    let mut path_var = env::var("PATH").unwrap_or_default();
    // smallweb likes to remove all ENV vars
    // put PATH back
    if path_var.is_empty() {
        if let Ok(output) = Command::new("/bin/bash")
            .arg("--login")
            .arg("-c")
            .arg("echo $PATH")
            .output()
        {
            if output.status.success() {
                path_var = String::from_utf8_lossy(&output.stdout).trim().to_string();
                env::set_var("PATH", &path_var);
            }
        }
    }

    let (action, _own_abs_path) = decide_action(&args, &path_var);
    match action {
        Action::Exec(config, deno_args) => {
            let bwrap_args = linux::deno_sandbox_to_bubblewrap_args(&args);
            let mut command = Command::new("bwrap");
            command.args(&bwrap_args);
            command.arg("--");
            command.arg("/bin/sh");
            command.env("PORT", deno_args.port.to_string());
            let shell_script = format!("set -x\n{}", &config.exec);
            debug_log!("Spawning command: {:?}", &command);
            spawn_and_wait_for_port(&mut command, deno_args.port, Some(&shell_script));
        }
        Action::ExecDeno { new_path } => {
            let mut command = Command::new("deno");
            command.args(&args[1..]);
            if let Some(p) = new_path {
                command.env("PATH", p);
            }
            debug_log!("Executing command: {:?}", &command);
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
        let temp_dir = tempdir().unwrap();
        let fake_deno_path = temp_dir.path().join("deno");
        std::fs::File::create(&fake_deno_path).unwrap();

        let file_path = std::path::Path::new("test/invoke_adapter/main.tsx");
        let absolute_path = std::fs::canonicalize(file_path).unwrap();
        let entrypoint = format!("file://{}", absolute_path.to_str().unwrap());
        let json_arg = format!(
            r#"{{"command":"fetch","entrypoint":"{}","port":38025}}"#,
            entrypoint
        );
        let args = vec![
            fake_deno_path.to_str().unwrap().to_string(),
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

        let (action, own_abs_path) = decide_action(&args, path_var);
        assert_eq!(
            own_abs_path,
            std::fs::canonicalize(&fake_deno_path).unwrap()
        );
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
        let temp_dir_adapter = tempdir().unwrap();
        let adapter_path = temp_dir_adapter.path().join("deno");
        std::fs::File::create(&adapter_path).unwrap();

        let args = vec![
            adapter_path.to_str().unwrap().to_string(),
            "run".to_string(),
            "--allow-net".to_string(),
            json_arg.clone(),
        ];

        let temp_dir = tempdir().unwrap();
        let deno_dir = temp_dir.path();
        std::fs::File::create(deno_dir.join("deno")).unwrap();

        let original_path = env::join_paths(
            [
                temp_dir_adapter.path(),
                deno_dir,
                Path::new("/usr/bin"),
                Path::new("/bin"),
            ]
            .iter(),
        )
        .unwrap();

        let (action, own_abs_path) = decide_action(&args, original_path.to_str().unwrap());
        assert_eq!(
            own_abs_path,
            std::fs::canonicalize(&adapter_path).unwrap()
        );

        let expected_new_path =
            env::join_paths([deno_dir, Path::new("/usr/bin"), Path::new("/bin")].iter()).unwrap();

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
        let temp_dir_adapter = tempdir().unwrap();
        let adapter_path = temp_dir_adapter.path().join("deno");
        std::fs::File::create(&adapter_path).unwrap();

        let args = vec![
            adapter_path.to_str().unwrap().to_string(),
            "run".to_string(),
            "-".to_string(),
            json_arg.clone(),
        ];

        let temp_dir = tempdir().unwrap();
        let deno_dir = temp_dir.path();
        std::fs::File::create(deno_dir.join("deno")).unwrap();

        let original_path = env::join_paths(
            [
                temp_dir_adapter.path(),
                deno_dir,
                Path::new("/usr/bin"),
                Path::new("/bin"),
            ]
            .iter(),
        )
        .unwrap();

        let (action, own_abs_path) = decide_action(&args, original_path.to_str().unwrap());
        assert_eq!(
            own_abs_path,
            std::fs::canonicalize(&adapter_path).unwrap()
        );

        let expected_new_path =
            env::join_paths([deno_dir, Path::new("/usr/bin"), Path::new("/bin")].iter()).unwrap();

        assert_eq!(
            action,
            Action::ExecDeno {
                new_path: Some(expected_new_path)
            }
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_path_canonicalization_with_symlink() {
        // This test ensures that our PATH manipulation logic correctly handles
        // canonicalization, for example when a path contains symlinks.

        // 1. Setup:
        // - A directory for the real deno: /tmp/real_deno/deno
        // - A directory for our adapter: /tmp/adapter/deno
        // - A symlink to our adapter's directory: /tmp/symlink_dir -> /tmp/adapter
        let temp_dir = tempdir().unwrap();
        let real_deno_dir = temp_dir.path().join("real_deno");
        std::fs::create_dir(&real_deno_dir).unwrap();
        std::fs::File::create(real_deno_dir.join("deno")).unwrap();

        let adapter_dir = temp_dir.path().join("adapter");
        std::fs::create_dir(&adapter_dir).unwrap();
        let adapter_path = adapter_dir.join("deno");
        std::fs::File::create(&adapter_path).unwrap();

        let symlink_dir = temp_dir.path().join("symlink_dir");
        std::os::unix::fs::symlink(&adapter_dir, &symlink_dir).unwrap();

        // 2. Construct PATH:
        // The PATH will contain the symlinked directory first, then the real deno dir.
        // Our logic should remove the symlinked entry and keep the real one.
        // We also add the symlink path with a trailing slash to test that case.
        let symlink_dir_with_slash = PathBuf::from(format!("{}/", symlink_dir.to_str().unwrap()));
        let original_path = env::join_paths(
            [
                &symlink_dir,
                &symlink_dir_with_slash,
                &real_deno_dir,
                Path::new("/usr/bin"),
            ]
            .iter(),
        )
        .unwrap();

        let args = vec![
            adapter_path.to_str().unwrap().to_string(),
            "run".to_string(),
            "foo.ts".to_string(),
        ];

        // 3. Run decide_action and assert
        let (action, own_abs_path) = decide_action(&args, original_path.to_str().unwrap());
        assert_eq!(
            own_abs_path,
            std::fs::canonicalize(&adapter_path).unwrap()
        );

        // The symlink_dir should be removed from the path, leaving real_deno_dir.
        let expected_new_path =
            env::join_paths([&real_deno_dir, Path::new("/usr/bin")].iter()).unwrap();

        assert_eq!(
            action,
            Action::ExecDeno {
                new_path: Some(expected_new_path)
            }
        );
    }
}
