use std::{fs, collections::HashMap, path::{Path, PathBuf}, process::exit, io::Write};
use anyhow::Result;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use clap::{crate_name, Parser};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

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

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let mut matched_color = ColorSpec::new();
    matched_color.set_fg(Some(Color::Green));
    matched_color.set_bold(true);
    let mut unmatched_color = ColorSpec::new();
    unmatched_color.set_bold(true);
    let mut path_color = ColorSpec::new();
    path_color.set_dimmed(true);

    let mut n_found = 0;

    for dir in &category.dirs {
        log::debug!("Searching in dir: {}", dir);
        let paths = walk_dir(dir);
        log::debug!("Found {} paths", paths.len());
        for path in paths {
            let filename = path.file_name().ok_or(Error::CouldntGetFileName(path.clone()))?;
            let filename = filename.to_string_lossy();
            if filename.starts_with(&term) {
                let matched_str = &filename[0..term.len()];
                let unmatched_str = &filename[term.len()..];

                stdout.set_color(&matched_color)?;
                write!(&mut stdout, "{}", matched_str)?;
                stdout.set_color(&unmatched_color)?;
                write!(&mut stdout, "{}", unmatched_str)?;
                stdout.set_color(&path_color)?;
                writeln!(&mut stdout, " ({})", path.display())?;
                stdout.reset()?;
                n_found += 1;
            }
        }
    }
    println!("Found {} files", n_found);
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

