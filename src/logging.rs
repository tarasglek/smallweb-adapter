use std::env;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static LOG_FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();

// Lazily initializes a log file. If the file can't be opened (e.g., due to permissions
// or the directory not existing), logging will be silently disabled for the
// remainder of the process's lifetime.
fn get_log_file() -> &'static Mutex<Option<File>> {
    LOG_FILE.get_or_init(|| {
        let file = env::var("SMALLWEB_APP_DIR").ok().and_then(|app_dir| {
            let path = Path::new(&app_dir)
                .join("logs")
                .join("smallweb-wrapper.log");
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok()
        });
        Mutex::new(file)
    })
}

pub fn log_internal(args: fmt::Arguments) {
    if let Ok(mut guard) = get_log_file().lock() {
        if let Some(file) = guard.as_mut() {
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
