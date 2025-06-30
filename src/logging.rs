use std::env;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static LOG_FILE_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

fn get_log_path() -> &'static Option<PathBuf> {
    LOG_FILE_PATH.get_or_init(|| {
        env::var("SMALLWEB_APP_DIR").ok().map(|app_dir| {
            Path::new(&app_dir)
                .join("logs")
                .join("smallweb-wrapper.log")
        })
    })
}

pub fn log_internal(args: fmt::Arguments) {
    if let Some(path) = get_log_path() {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
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
