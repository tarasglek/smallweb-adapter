use std::env;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn log_internal(args: fmt::Arguments) {
    if let Ok(app_dir) = env::var("SMALLWEB_APP_DIR") {
        let log_dir = Path::new(&app_dir).join("logs");
        if fs::create_dir_all(&log_dir).is_err() {
            return;
        }
        let log_file_path = log_dir.join("smallweb-wrapper.log");

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
        {
            let _ = writeln!(file, "{}", args);
        }
    }
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        $crate::logging::log_internal(format_args!($($arg)*))
    };
}
