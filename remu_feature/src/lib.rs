use std::{env, fs, path::PathBuf};

use pest::Parser;

#[derive(pest_derive::Parser)]
#[grammar = "src/config_parser.pest"]
struct ConfigParser;

fn find_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut current_dir = manifest_dir.as_path();

    loop {
        let potential_ws_toml = current_dir.join("Cargo.toml");
        if potential_ws_toml.is_file() {
            if let Ok(contents) = fs::read_to_string(&potential_ws_toml) {
                if contents.contains("[workspace]") {
                    return current_dir.to_path_buf();
                }
            }
        }

        match current_dir.parent() {
            Some(parent) => current_dir = parent,
            None => panic!(
                "Error: Could not find workspace root from '{}'. Traversed up to filesystem root.",
                manifest_dir.display()
            ),
        }
    }
}

fn find_config_path() -> PathBuf {
    let workspace_root = find_workspace_root();
    let config_path = workspace_root
        .join("config")
        .join("static")
        .join(".config");

    if !config_path.is_file() {
        panic!(
            "Error: Configuration file not found at the expected path: {}",
            config_path.display()
        );
    }

    config_path
}

pub fn apply_features() {
    let config_path = find_config_path();

    println!("cargo:rerun-if-changed={}", config_path.display());

    let config_content = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read config file at {}: {}", config_path.display(), e));

    let pairs = ConfigParser::parse(Rule::file, &config_content)
        .unwrap_or_else(|e| panic!("Failed to parse config file: {}", e));

    for pair in pairs {
        if let Rule::config_key = pair.as_rule() {
            let feature_name = pair.as_str();
            println!("cargo:rustc-cfg=feature=\"{}\"", feature_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_features() {
        apply_features();
    }
}
