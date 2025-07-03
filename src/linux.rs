use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Command;

pub fn is_port_listening(port: u16) -> bool {
    debug_log!("[netstat] checking for port {}", port);
    let output = match Command::new("netstat").arg("-tln").output() {
        Ok(output) => output,
        Err(e) => {
            debug_log!("[netstat] failed to run: {}", e);
            // netstat might not be installed, or we are on a system that doesn't have it.
            // We can't check, so we'll have to assume it's not listening, or handle this case diff
            // For now, let's assume it's not listening if we can't run netstat.
            return false;
        }
    };

    if !output.status.success() {
        debug_log!("[netstat] failed with status: {}", output.status);
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains(&format!(":{}", port)) {
            debug_log!("[netstat] found port {} in use", port);
            return true;
        }
    }

    debug_log!("[netstat] port {} not in use", port);
    false
}

fn bind_mount(path: &str, rw: bool) -> Option<[String; 3]> {
    if !Path::new(path).exists() {
        debug_log!("skipping bind mount for non-existent path: {}", path);
        return None;
    }
    let flag = if rw { "--bind" } else { "--ro-bind" };
    Some([flag.to_string(), path.to_string(), path.to_string()])
}

pub fn deno_sandbox_to_bubblewrap_args(args: &[String], own_path: &Path) -> Vec<String> {
    let mut bwrap_args: Vec<String> = [
        "--die-with-parent", "--unshare-pid", "--new-session",
        "--proc", "/proc", "--dev", "/dev",
        "--symlink", "usr/lib64", "/lib64",
    ]
    .into_iter().map(String::from).collect();

    bwrap_args.extend(
        ["/bin", "/usr", "/lib"]
            .iter()
            .flat_map(|&path| bind_mount(path, false))
            .flatten(),
    );

    if args.iter().any(|arg| arg == "--allow-net") {
        bwrap_args.push("--share-net".to_string());
        bwrap_args.extend(
            ["/etc/resolv.conf", "/etc/ssl"]
                .iter()
                .flat_map(|&path| bind_mount(path, false))
                .flatten(),
        );
    }

    let own_meta = own_path.metadata().ok();

    let should_bind = |path_str: &str| -> bool {
        if let Some(own_meta) = &own_meta {
            let path = Path::new(path_str);
            if let Ok(p_meta) = path.metadata() {
                if p_meta.dev() == own_meta.dev() && p_meta.ino() == own_meta.ino() {
                    debug_log!("skipping bind mount for own path: {}", path_str);
                    return false;
                }
            }
        }
        true
    };

    let read_args = args.iter()
        .filter_map(|arg| arg.strip_prefix("--allow-read="))
        .flat_map(|paths| paths.split(','))
        .filter(|path| !path.is_empty())
        .filter(|path| should_bind(path))
        .flat_map(|path| bind_mount(path, false))
        .flatten();

    let write_args = args.iter()
        .filter_map(|arg| arg.strip_prefix("--allow-write="))
        .flat_map(|paths| paths.split(','))
        .filter(|path| !path.is_empty())
        .filter(|path| should_bind(path))
        .flat_map(|path| bind_mount(path, true))
        .flatten();

    bwrap_args.extend(read_args);
    bwrap_args.extend(write_args);

    bwrap_args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    fn to_string_vec(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_basic_args() {
        let args = vec![];
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args, Path::new("/fake/deno"));
        assert!(bwrap_args.contains(&"--die-with-parent".to_string()));
        assert!(bwrap_args.windows(3).any(|w| w == ["--ro-bind", "/bin", "/bin"]));
        assert!(!bwrap_args.contains(&"--share-net".to_string()));
    }

    #[test]
    fn test_with_allow_net() {
        let args = to_string_vec(&["--allow-net"]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args, Path::new("/fake/deno"));
        assert!(bwrap_args.contains(&"--share-net".to_string()));
        assert!(bwrap_args.windows(3).any(|w| w == ["--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf"]));
    }

    #[test]
    fn test_with_allow_read() {
        let temp_dir = tempdir().unwrap();
        let home_user = temp_dir.path().join("user");
        std::fs::create_dir(&home_user).unwrap();
        let home_user_str = home_user.to_str().unwrap();

        let args = to_string_vec(&[&format!("--allow-read={0},/tmp", home_user_str)]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args, Path::new("/fake/deno"));
        assert!(bwrap_args
            .windows(3)
            .any(|w| w == ["--ro-bind", home_user_str, home_user_str]));
        assert!(bwrap_args
            .windows(3)
            .any(|w| w == ["--ro-bind", "/tmp", "/tmp"]));
    }

    #[test]
    fn test_with_allow_read_non_existent() {
        let temp_dir = tempdir().unwrap();
        let non_existent_path = temp_dir.path().join("non_existent");
        let non_existent_path_str = non_existent_path.to_str().unwrap();
        let args = to_string_vec(&[&format!("--allow-read={}", non_existent_path_str)]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args, Path::new("/fake/deno"));
        assert!(!bwrap_args
            .windows(3)
            .any(|w| w == ["--ro-bind", non_existent_path_str, non_existent_path_str]));
    }

    #[test]
    fn test_with_allow_write() {
        let temp_dir = tempdir().unwrap();
        let data_dir = temp_dir.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();
        let data_dir_str = data_dir.to_str().unwrap();

        let args = to_string_vec(&[&format!("--allow-write={}", data_dir_str)]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args, Path::new("/fake/deno"));
        assert!(bwrap_args
            .windows(3)
            .any(|w| w == ["--bind", data_dir_str, data_dir_str]));
    }

    #[test]
    fn test_with_mixed_args() {
        let temp_dir = tempdir().unwrap();
        let home_user = temp_dir.path().join("user");
        std::fs::create_dir(&home_user).unwrap();
        let home_user_str = home_user.to_str().unwrap();
        let data_dir = temp_dir.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();
        let data_dir_str = data_dir.to_str().unwrap();

        let args = to_string_vec(&[
            "--allow-net",
            &format!("--allow-read={}", home_user_str),
            &format!("--allow-write={}", data_dir_str),
        ]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args, Path::new("/fake/deno"));
        assert!(bwrap_args.contains(&"--share-net".to_string()));
        assert!(bwrap_args
            .windows(3)
            .any(|w| w == ["--ro-bind", home_user_str, home_user_str]));
        assert!(bwrap_args
            .windows(3)
            .any(|w| w == ["--bind", data_dir_str, data_dir_str]));
    }

    #[test]
    fn test_filter_own_path() {
        let temp_dir = tempdir().unwrap();
        let own_path_dir = temp_dir.path();
        let own_path = own_path_dir.join("deno");
        std::fs::File::create(&own_path).unwrap();

        let data_dir = temp_dir.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();
        let data_dir_str = data_dir.to_str().unwrap();

        let own_path_str = own_path.to_str().unwrap();
        let args = to_string_vec(&[
            &format!("--allow-read={}", own_path_str),
            &format!("--allow-write={}", data_dir_str),
        ]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args, &own_path);

        // Should not contain the bind mount for its own path
        assert!(!bwrap_args
            .windows(3)
            .any(|w| w == ["--ro-bind", own_path_str, own_path_str]));
        // Should still contain the other bind mount
        assert!(bwrap_args
            .windows(3)
            .any(|w| w == ["--bind", data_dir_str, data_dir_str]));
    }
}
