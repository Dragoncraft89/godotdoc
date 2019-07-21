extern crate ansi_term;
extern crate clap;
extern crate glob;
extern crate serde;
extern crate serde_json;

use crate::backend::markdownbackend::MarkdownBackend;
use crate::backend::Backend;

use ansi_term::Colour::Red;
use clap::{App, Arg};
use serde::Deserialize;

use glob::Pattern;

use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

mod backend;
mod parser;

use crate::parser::parse_file;

fn handle_error<T>(x: Result<T, String>) -> T {
    match x {
        Ok(y) => y,
        Err(e) => {
            eprintln!("{}", Red.paint(format!("Error: {}", e)));
            ::std::process::exit(1);
        }
    }
}

#[derive(Default, Deserialize)]
struct Configuration {
    backend: Option<String>,
    excluded_files: Option<Vec<String>>,
}

struct Settings<'a> {
    backend: Box<Backend>,
    output_path: &'a Path,

    excluded_files: Vec<Pattern>,
}

fn main() {
    let matches = App::new("Godot Doc")
        .version("1.0")
        .author("Florian Kothmeier <floriankothmeier@web.de>")
        .about("Documentation generator for Gdscript")
        .arg(
            Arg::with_name("backend")
                .help("Sets the type of file, which will be generated")
                .long("backend")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .help("Sets the directory to output files")
                .short("o")
                .long("output")
                .value_name("Directory")
                .required(true),
        )
        .arg(Arg::with_name("input directory").required(true).index(1))
        .get_matches();

    let input_dir = matches.value_of("input directory").unwrap();
    let output_dir = matches.value_of("output").unwrap();
    let config;
    if let Ok(f) = File::open(Path::new(input_dir).join("godotdoc_config.json")) {
        config = handle_error(
            serde_json::from_reader(f).map_err(|e| format!("Error config file: {}", e)),
        );
    } else {
        config = Configuration::default();
    }

    let config_backend = config.backend.as_ref().map(|s| s.as_str());
    let backend: Box<Backend> =
        handle_error(get_backend(matches.value_of("backend").or(config_backend)));

    let settings = Settings {
        backend: backend,
        output_path: Path::new(output_dir),

        excluded_files: config
            .excluded_files
            .unwrap_or(Vec::new())
            .drain(..)
            .map(|s| handle_error(Pattern::new(s.as_str()).map_err(|e| e.to_string())))
            .collect(),
    };
    handle_error(traverse_directory(
        Path::new(input_dir).to_path_buf(),
        Path::new(".").to_path_buf(),
        &settings,
    ))
}

fn get_backend(name: Option<&str>) -> Result<Box<Backend>, String> {
    match name {
        Some("markdown") | None => Ok(Box::new(MarkdownBackend::new())),
        _ => Err("Unsupported backend".to_string()),
    }
}

fn path_matches_any(path: &Path, patterns: &Vec<Pattern>) -> bool {
    for pattern in patterns {
        if pattern.matches_path(path) {
            return true;
        }
    }

    return false;
}

fn traverse_directory(src: PathBuf, output: PathBuf, settings: &Settings) -> Result<(), String> {
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        let file_name = path.file_name().map(|e| e.to_str().unwrap());

        let new_output = Path::new(&output).join(file_name.unwrap());
        if path_matches_any(&new_output, &settings.excluded_files) {
            continue;
        }

        if path.is_dir() {
            traverse_directory(path, new_output, settings)?;
        } else if path.is_file() && path.extension() == Some(OsStr::new("gd")) {
            let input = File::open(&path)
                .map_err(|e| format!("Failed to open input file: {}, {}", path.display(), e))?;
            let output_path = settings.output_path.join(&output).join(format!(
                "{}.{}",
                file_name.unwrap(),
                settings.backend.get_extension()
            ));

            std::fs::create_dir_all(&output_path.parent().unwrap()).map_err(|e| e.to_string())?;
            let mut output = File::create(&output_path).map_err(|e| {
                format!(
                    "Failed to open output file: {}, {}",
                    output_path.display(),
                    e
                )
            })?;
            settings
                .backend
                .generate_output(parse_file(file_name.unwrap(), input)?, &mut output)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
