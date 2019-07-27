pub mod validation;

use std::fs;

use crate::config::Config;

pub fn run(config: Config) {
    match config {
        Config::Validate { file } => {
            validate(file)
        }
        _ => unimplemented!()
    }
}

fn validate(file: String) {
    let text = fs::read_to_string(&file).unwrap();

    let root_table: toml::value::Table = toml::from_str(&text).unwrap();
    let r = validation::check_root_table(root_table);

    match r {
        Ok(_) => {
            println!("Validation completed successfully for file '{}'", &file);
        },
        Err(errors) => {
            for e in &errors {
                println!("{}\n", e);
            }

            if errors.len() == 1 {
                println!("error: aborting due to previous error");
            } else {
                println!("error: aborting due to {} previous errors", errors.len());
            }

            println!("\nerror: Could not validate file '{}'\n", &file);

            std::process::exit(-1);
        }
    }
}
