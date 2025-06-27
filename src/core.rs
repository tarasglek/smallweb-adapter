use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::fs;

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // command and port are unused for now
pub struct DenoArgs {
    command: String,
    entrypoint: String,
    port: u16,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // Fields are not used, we only care about parsing success.
pub struct MainTsxConfig {
    watchpattern: String,
    exec: String,
    build: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Print(String),
    ExecDeno { new_path: Option<OsString> },
}

pub fn decide_action(args: &[String], path_var: &str) -> Action {
    let should_change_path = args.get(0).map_or(false, |a| a.ends_with("deno"));

    let create_new_path = || {
        if should_change_path {
            let mut paths: Vec<_> = env::split_paths(path_var).collect();
            if !paths.is_empty() {
                paths.remove(0);
            }
            Some(env::join_paths(paths).unwrap())
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
        return fallback();
    };

    let deno_args = if let Ok(args) = serde_json::from_str::<DenoArgs>(last_arg) {
        args
    } else {
        return fallback();
    };

    let path_str = if let Some(p) = deno_args.entrypoint.strip_prefix("file://") {
        p
    } else {
        return fallback();
    };

    if !path_str.ends_with("main.tsx") {
        return fallback();
    }

    let file_content = if let Ok(content) = fs::read_to_string(path_str) {
        content
    } else {
        return fallback();
    };

    if !file_content.starts_with('{') {
        return fallback();
    }

    if serde_json::from_str::<MainTsxConfig>(&file_content).is_ok() {
        Action::Print(file_content)
    } else {
        fallback()
    }
}
