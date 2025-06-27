use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::fs;

macro_rules! debug_log {
    ($($arg:tt)*) => {
        if env::var("DEBUG").is_ok() {
            eprintln!($($arg)*);
        }
    };
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[allow(dead_code)] // command is unused for now
pub struct DenoArgs {
    pub command: String,
    pub entrypoint: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct MainTsxConfig {
    pub exec: String,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Exec(MainTsxConfig, DenoArgs),
    ExecDeno { new_path: Option<OsString> },
}

pub fn decide_action(args: &[String], path_var: &str) -> Action {
    debug_log!("decide_action called with args: {:?}", args);
    debug_log!("original PATH: {}", path_var);
    let should_change_path = args.get(0).map_or(false, |a| a.ends_with("deno"));
    debug_log!("should_change_path: {}", should_change_path);

    let create_new_path = || {
        if should_change_path {
            let mut paths: Vec<_> = env::split_paths(path_var).collect();
            if !paths.is_empty() {
                paths.remove(0);
            }
            let new_path = env::join_paths(paths).unwrap();
            debug_log!("new PATH: {:?}", new_path);
            Some(new_path)
        } else {
            None
        }
    };

    let fallback = || Action::ExecDeno {
        new_path: create_new_path(),
    };

    let last_arg = if let Some(arg) = args.last() {
        arg
    } else {
        debug_log!("No last arg, falling back.");
        return fallback();
    };
    debug_log!("last_arg: {}", last_arg);

    let deno_args = if let Ok(args) = serde_json::from_str::<DenoArgs>(last_arg) {
        args
    } else {
        debug_log!("Failed to parse last_arg as DenoArgs, falling back.");
        return fallback();
    };
    debug_log!("deno_args: {:?}", deno_args);

    if let Some(path_str) = deno_args.entrypoint.strip_prefix("file://") {
        debug_log!("path_str: {}", path_str);

        if !path_str.ends_with("main.tsx") {
            debug_log!("path_str doesn't end with main.tsx, falling back.");
            return fallback();
        }

        let file_content = if let Ok(content) = fs::read_to_string(path_str) {
            content
        } else {
            debug_log!("Failed to read file content from path_str, falling back.");
            return fallback();
        };
        debug_log!("file_content: {}", file_content);

        if !file_content.starts_with('{') {
            debug_log!("file_content doesn't start with '{{', falling back.");
            return fallback();
        }

        if let Ok(config) = serde_json::from_str::<MainTsxConfig>(&file_content) {
            debug_log!(
                "Successfully parsed file_content as MainTsxConfig, returning Action::Exec."
            );
            Action::Exec(config, deno_args)
        } else {
            debug_log!("Failed to parse file_content as MainTsxConfig, falling back.");
            fallback()
        }
    } else {
        debug_log!(
            "entrypoint {} doesn't start with file://, falling back.",
            deno_args.entrypoint
        );
        fallback()
    }
}
