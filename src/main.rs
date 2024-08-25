use std::{fs, collections::HashMap, path::{Path, PathBuf}, process::exit};
use anyhow::Result;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use clap::{crate_name, Parser};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Config directory not found")]
    ConfigDirNotFound,
    #[error("Search category not found: {0}")]
    CategoryNotFound(String),
    #[error("Could not get file name for path: {0}")]
    CouldntGetFileName(PathBuf),
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(flatten)]
    categories: HashMap<String, CategoryConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config { categories: HashMap::new() }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct CategoryConfig {
    dirs: Vec<String>,
}

#[derive(Parser)]
struct Opts {
    search_category: String,
    search_term: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let config = prepare_config()?;
    log::debug!("Config: {:#?}", config);

    let opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(_) => {
            let categories = config.categories.keys().cloned().collect::<Vec<_>>().join(", ");
            eprintln!("Usage: prefix-search [{categories}] <SEARCH_TERM>");
            exit(1);
        }
    };
    let category = config.categories.get(&opts.search_category).ok_or(Error::CategoryNotFound(opts.search_category))?;
    let term = opts.search_term;
    for dir in &category.dirs {
        log::debug!("Searching in dir: {}", dir);
        let paths = walk_dir(dir);
        log::debug!("Found {} paths", paths.len());
        for path in paths {
            let filename = path.file_name().ok_or(Error::CouldntGetFileName(path.clone()))?;
            let filename = filename.to_string_lossy();
            if filename.starts_with(&term) {
                println!("{}", path.display());
            }
        }
    }
    Ok(())
}

fn prepare_config() -> Result<Config> {
    let config_parent_dir = config_dir().ok_or(Error::ConfigDirNotFound)?;
    let config_dir = config_parent_dir.join(crate_name!());
    fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("config.toml");
    if !config_path.exists() {
        let default_config = Config::default();
        let toml = toml::to_string_pretty(&default_config)?;
        std::fs::write(&config_path, toml)?;
        log::info!("Default config written to {:?}", config_path);
    }
    let config = config::Config::builder()
        .add_source(config::File::from(config_path))
        .build()?;
    let config = config.try_deserialize::<Config>()?;

    Ok(config)
}

fn walk_dir(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let iter = match fs::read_dir(dir.as_ref()) {
        Ok(iter) => iter,
        Err(err) => {
            eprintln!("Ignoring error {} in {}", err, dir.as_ref().display());
            return paths;
        }
    };
    for entry in iter {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("Ignoring error {} in {}", err, dir.as_ref().display());
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            paths.extend(walk_dir(&path));
        } else {
            paths.push(path);
        }
    }
    paths
}

