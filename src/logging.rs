use std::env;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn log_internal(args: fmt::Arguments) {
    let debug_val = match env::var("DEBUG") {
        Ok(val) => val,
        Err(_) => {
            if let Ok(app_dir) = env::var("SMALLWEB_APP_DIR") {
                let log_dir = Path::new(&app_dir).join("data");
                let _ = fs::create_dir_all(&log_dir);
                log_dir
                    .join("wrapper.log")
                    .to_string_lossy()
                    .into_owned()
            } else {
                // Fallback to stderr if SMALLWEB_APP_DIR is not set
                "".to_string()
            }
        }
    };

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
