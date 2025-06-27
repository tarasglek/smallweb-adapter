use std::process::Command;

pub fn is_port_listening(port: u16) -> bool {
    let output = match Command::new("netstat").arg("-n").output() {
        Ok(output) => output,
        Err(_) => {
            // netstat might not be installed, or we are on a system that doesn't have it.
            // We can't check, so we'll have to assume it's not listening, or handle this case differently.
            // For now, let's assume it's not listening if we can't run netstat.
            return false;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let port_str = format!(":{}", port);

    // This is a simple check. It might produce false positives if the port number
    // appears elsewhere in the line. A more robust solution would parse the output.
    stdout.lines().any(|line| line.contains(&port_str) && line.contains("LISTEN"))
}
