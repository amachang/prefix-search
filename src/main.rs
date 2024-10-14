use std::{collections::{HashMap, HashSet}, path::PathBuf, process::exit, io::Write};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use clap::{crate_name, Parser};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use serde_json::{Map, Value, Number};

#[derive(Debug, thiserror::Error)]
enum Error {
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
    #[clap(required = true)]
    search_terms: Vec<String>,
    #[clap(short, long, help = "To use the command in shell's if-else condition")]
    question: bool,
    #[clap(long, help = "Output result as json")]
    json: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    jdt::use_from(crate_name!());
    let config = jdt::config::<Config>();

    log::debug!("Config: {:#?}", config);

    let opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(_) => {
            let categories = config.categories.keys().cloned().collect::<Vec<_>>().join(", ");
            eprintln!("Usage: prefix-search [{categories}] [-q] <SEARCH_TERM> [<SEARCH_TERM>...]");
            exit(1);
        }
    };
    let quiet = opts.question;
    let use_failed_exit_code_if_no_match = opts.question;
    let only_first_match = opts.question;
    let outputs_json = opts.json;

    let category = config.categories.get(&opts.search_category).ok_or(Error::CategoryNotFound(opts.search_category))?;
    let mut seen_terms = HashSet::new();
    let mut terms = opts.search_terms;
    // longest term first
    terms.sort_by(|a, b| b.len().cmp(&a.len()));

    let mut n_found = 0;
    let mut term_filename_path_list = Vec::new();
    for dir in &category.dirs {
        let paths = jdt::walk_dir(dir, |path| path);
        for path in paths {
            let filename = path.file_name().ok_or(Error::CouldntGetFileName(path.clone()))?;
            let filename = filename.to_string_lossy().to_string();
            for term in &terms {
                if filename.starts_with(&*term) {
                    term_filename_path_list.push((term.clone(), filename.clone(), path.clone()));
                    n_found += 1;
                    seen_terms.insert(term.clone());
                    break;
                }
            }
            if only_first_match && n_found > 0 {
                break;
            }
        }
    }
    term_filename_path_list.sort_by(|a, b| a.2.cmp(&b.2));

    let unseen_terms = terms.into_iter().filter(|term| !seen_terms.contains(term));
    let unseen_terms = unseen_terms.collect::<HashSet<_>>();
    let mut sorted_unseen_terms = unseen_terms.iter().cloned().collect::<Vec<_>>();
    sorted_unseen_terms.sort();

    if !quiet {
        if outputs_json {
            let mut json = Map::new();
            json.insert("nFound".to_string(), Value::Number(Number::from(n_found)));
            let term_filename_path_list = term_filename_path_list.into_iter().map(|(term, filename, path)| {
                let mut map = Map::new();
                map.insert("term".to_string(), Value::String(term));
                map.insert("filename".to_string(), Value::String(filename));
                map.insert("path".to_string(), Value::String(path.display().to_string()));
                Value::Object(map)
            }).collect::<Vec<_>>();
            json.insert("foundPathList".to_string(), Value::Array(term_filename_path_list));
            let unseen_terms = sorted_unseen_terms.into_iter().map(|term| Value::String(term)).collect::<Vec<_>>();
            json.insert("unseenTermList".to_string(), Value::Array(unseen_terms));
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            let mut stdout = StandardStream::stdout(ColorChoice::Always);
            let mut matched_color = ColorSpec::new();
            matched_color.set_fg(Some(Color::Green));
            matched_color.set_bold(true);
            let mut unmatched_color = ColorSpec::new();
            unmatched_color.set_bold(true);
            let mut path_color = ColorSpec::new();
            path_color.set_dimmed(true);

            for (term, filename, path) in term_filename_path_list {
                stdout.set_color(&matched_color)?;
                write!(&mut stdout, "{}", term)?;
                stdout.set_color(&unmatched_color)?;
                write!(&mut stdout, "{}", &filename[term.len()..])?;
                stdout.set_color(&path_color)?;
                write!(&mut stdout, " ({})", path.display())?;
                stdout.reset()?;
                writeln!(&mut stdout)?;
            }

            println!("Found {} files", n_found);
            if !unseen_terms.is_empty() {
                println!("Unmet search terms: {}", sorted_unseen_terms.join(" "));
            }
        }
    }

    if use_failed_exit_code_if_no_match {
        if n_found > 0 {
            exit(0);
        } else {
            exit(1);
        }
    }

    Ok(())
}

