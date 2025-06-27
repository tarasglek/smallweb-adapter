use std::env;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;

pub fn log_internal(args: fmt::Arguments) {
    let debug_val = env::var("DEBUG").unwrap_or_else(|_| "/etc/smallweb-wrapper.log".to_string());
    if debug_val.contains('.') {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&debug_val)
        {
            let _ = writeln!(file, "{}", args);
        }
    } else {
        eprintln!("{}", args);
    }
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        $crate::logging::log_internal(format_args!($($arg)*))
    };
}
