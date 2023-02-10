mod base_config;
use anyhow::anyhow;
pub use base_config::Config;
use std::path::{Path, PathBuf};

pub mod cache_service_config;

pub fn parse_config(input: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(input)
}

pub fn load_config_file(
    path: &Option<&Path>,
    cfg_name_for_fallbacks: &str,
) -> Result<Config, Box<dyn std::error::Error>> {
    let mut cfg_try_paths: Vec<PathBuf> = vec![];

    if let Some(p) = &path {
        if !p.exists() {
            return Err(anyhow!(
                "Expected to find config at path {}, but it didn't exist",
                p.to_string_lossy()
            )
            .into());
        }
        cfg_try_paths.push(p.to_path_buf());
    };

    if let Ok(home_dir) = std::env::var("HOME") {
        cfg_try_paths.push(PathBuf::from(format!(
            "{}/.{}",
            home_dir, cfg_name_for_fallbacks
        )));
    }

    cfg_try_paths.push(PathBuf::from(format!("/etc/.{}", cfg_name_for_fallbacks)));

    for path in cfg_try_paths.into_iter() {
        if path.exists() {
            return Ok(parse_config(&std::fs::read_to_string(path)?)?);
        }
    }
    Ok(Config::default())
}
