pub mod validation;
pub mod codegen;

use std::fs;

use validation::{ValidationError, ParsedFile};
use crate::config::{ Config, Language };

pub fn run(config: Config) {
    match config {
        Config::Validate { file } => {
            validate(file)
        },
        Config::Edit { file } => {
            edit(file)
        }
        Config::Generate {input, output, language } => {
            generate(input, output, language)
        }
        _ => unimplemented!()
    }
}

fn validate(file_path: String) {
    let r = run_validation_and_print_errors(&file_path);

    match r {
        Ok(_) => println!("Validation completed successfully for file '{}'", &file_path),
        Err(_) => std::process::exit(-1),
    }
}

fn run_validation(file_path: &str) -> Result<(ParsedFile, String), Vec<ValidationError>> {
    let text = fs::read_to_string(&file_path).unwrap();

    let root_table: toml::value::Table = toml::from_str(&text).unwrap();
    validation::check_root_table(root_table).map(|f| (f, text))
}

fn run_validation_and_print_errors(file_path: &str) -> Result<(ParsedFile, String), Vec<ValidationError>> {
    let r = run_validation(&file_path);

    if let Err(errors) = &r {
        for e in errors {
            println!("{}\n", e);
        }

        if errors.len() == 1 {
            println!("error: aborting due to previous error");
        } else {
            println!("error: aborting due to {} previous errors", errors.len());
        }

        println!("\nerror: Could not validate file '{}'\n", &file_path);
    }

    r
}


fn edit(file_path: String) {
    let (parsed_file, register_file_raw) = match run_validation_and_print_errors(&file_path) {
        Ok((parsed_file, raw)) => (parsed_file, raw),
        Err(_) => std::process::exit(-1),
    };

    crate::ui::run_ui(parsed_file, register_file_raw, file_path)
}

fn generate(input: String, output: String, language: Language) {
    let parsed_file = match run_validation_and_print_errors(&input) {
        Ok((parsed_file, _)) => parsed_file,
        Err(_) => std::process::exit(-1),
    };

    match language {
        Language::Rust => self::codegen::rust::parsed_file_to_rust(&parsed_file, &output)
    }
}
