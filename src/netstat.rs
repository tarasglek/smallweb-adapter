use std::env;
use std::process::Command;

pub fn is_port_listening(port: u16) -> bool {
    debug_log!("[netstat] checking for port {}", port);
    let output = match Command::new("netstat").arg("-tln").output() {
        Ok(output) => output,
        Err(e) => {
            debug_log!("[netstat] failed to run: {}", e);
            // netstat might not be installed, or we are on a system that doesn't have it.
            // We can't check, so we'll have to assume it's not listening, or handle this case differently.
            // For now, let's assume it's not listening if we can't run netstat.
            return false;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    debug_log!("[netstat] stdout:\n{}", stdout);
    let port_str = format!(":{} ", port);

    // This is a simple check. It might produce false positives if the port number
    // appears elsewhere in the line. A more robust solution would parse the output.
    let is_listening = stdout
        .lines()
        .any(|line| line.contains(&port_str) && line.contains("LISTEN"));
    debug_log!("[netstat] port {} listening: {}", port, is_listening);
    is_listening
}
