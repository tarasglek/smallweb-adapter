use crate::debug_log;
use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[allow(dead_code)] // command is unused for now
pub struct DenoArgs {
    pub command: String,
    pub entrypoint: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct SmallwebConfig {
    pub exec: String,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Exec(MainTsxConfig, DenoArgs),
    ExecDeno { new_path: Option<OsString> },
}

pub fn decide_action(args: &[String], path_var: &str) -> (Action, PathBuf) {
    debug_log!("decide_action called with args: {:?}", args);
    debug_log!("original PATH: {}", path_var);
    let own_abs_path = args
        .get(0)
        .and_then(|p| std::fs::canonicalize(p).ok())
        .expect("Failed to get absolute path of executable from args[0]");
    debug_log!("own_abs_path: {:?}", own_abs_path);

    let mut is_shadowing_deno = false;
    if let Some(executable_path) = args.get(0) {
        if let Some(file_name) = std::path::Path::new(executable_path).file_name() {
            is_shadowing_deno = file_name == "deno";
        }
    }
    debug_log!("is_shadowing_deno: {}", is_shadowing_deno);

    let create_new_path = || {
        if is_shadowing_deno {
            let mut paths: Vec<_> = env::split_paths(path_var).collect();
            let own_parent_dir = own_abs_path.parent();
            let parent_meta = own_parent_dir.and_then(|p| p.metadata().ok());

            paths.retain(|p| {
                if let Some(parent_meta) = &parent_meta {
                    if let Ok(p_meta) = p.metadata() {
                        if p_meta.dev() == parent_meta.dev() && p_meta.ino() == parent_meta.ino() {
                            debug_log!("removing path entry by metadata: {:?}", p);
                            return false;
                        }
                    }
                }
                true
            });
            let new_path = env::join_paths(paths).unwrap();
            debug_log!("new PATH: {:?}", new_path);
            Some(new_path)
        } else {
            None
        }
    };

    let fallback = || {
        (
            Action::ExecDeno {
                new_path: create_new_path(),
            },
            own_abs_path.clone(),
        )
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

        let entrypoint_path = PathBuf::from(path_str);
        let app_dir = if entrypoint_path.is_dir() {
            Some(entrypoint_path.as_path())
        } else {
            entrypoint_path.parent()
        };

        if let Some(dir) = app_dir {
            let config_path = dir.join("smallweb.json");
            debug_log!("checking for config file at {}", config_path.display());

            if let Ok(file_content) = fs::read_to_string(config_path) {
                debug_log!("file_content: {}", file_content);

                if let Ok(config) = serde_json::from_str::<SmallwebConfig>(&file_content) {
                    debug_log!(
                        "Successfully parsed file_content as SmallwebConfig, returning Action::Exec."
                    );
                    return (Action::Exec(config, deno_args), own_abs_path);
                } else {
                    debug_log!("Failed to parse file_content as SmallwebConfig, falling back.");
                }
            } else {
                debug_log!("Could not read config file, falling back.");
            }
        }
        fallback()
    } else {
        debug_log!(
            "entrypoint {} doesn't start with file://, falling back.",
            deno_args.entrypoint
        );
        fallback()
    }
}
