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

fn bind_mount(path: &str, rw: bool) -> [String; 3] {
    let flag = if rw { "--bind" } else { "--ro-bind" };
    [flag.to_string(), path.to_string(), path.to_string()]
}

pub fn deno_sandbox_to_bubblewrap_args(args: &[String]) -> Vec<String> {
    let mut bwrap_args: Vec<String> = [
        "--die-with-parent", "--unshare-pid", "--new-session",
        "--proc", "/proc", "--dev", "/dev",
        "--symlink", "usr/lib64", "/lib64",
    ]
    .into_iter().map(String::from).collect();

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

#[cfg(test)]
mod tests {
    use super::*;

    fn to_string_vec(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_basic_args() {
        let args = vec![];
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args);
        assert!(bwrap_args.contains(&"--die-with-parent".to_string()));
        assert!(bwrap_args.windows(3).any(|w| w == ["--ro-bind", "/bin", "/bin"]));
        assert!(!bwrap_args.contains(&"--share-net".to_string()));
    }

    #[test]
    fn test_with_allow_net() {
        let args = to_string_vec(&["--allow-net"]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args);
        assert!(bwrap_args.contains(&"--share-net".to_string()));
        assert!(bwrap_args.windows(3).any(|w| w == ["--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf"]));
    }

    #[test]
    fn test_with_allow_read() {
        let args = to_string_vec(&["--allow-read=/home/user,/tmp"]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args);
        assert!(bwrap_args.windows(3).any(|w| w == ["--ro-bind", "/home/user", "/home/user"]));
        assert!(bwrap_args.windows(3).any(|w| w == ["--ro-bind", "/tmp", "/tmp"]));
    }

    #[test]
    fn test_with_allow_write() {
        let args = to_string_vec(&["--allow-write=/data"]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args);
        assert!(bwrap_args.windows(3).any(|w| w == ["--bind", "/data", "/data"]));
    }

    #[test]
    fn test_with_mixed_args() {
        let args = to_string_vec(&[
            "--allow-net",
            "--allow-read=/home/user",
            "--allow-write=/data",
        ]);
        let bwrap_args = deno_sandbox_to_bubblewrap_args(&args);
        assert!(bwrap_args.contains(&"--share-net".to_string()));
        assert!(bwrap_args.windows(3).any(|w| w == ["--ro-bind", "/home/user", "/home/user"]));
        assert!(bwrap_args.windows(3).any(|w| w == ["--bind", "/data", "/data"]));
    }
}
