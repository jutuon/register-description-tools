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
    let text = fs::read_to_string(file).unwrap();

    let root_table: toml::value::Table = toml::from_str(&text).unwrap();
    let r = validation::check_root_table(root_table);

    match r {
        Ok(_) => (),
        Err(errors) => {
            println!("errors: {}\n{:#?}", errors.len(), errors);
            std::process::exit(-1);
        }
    }
}
