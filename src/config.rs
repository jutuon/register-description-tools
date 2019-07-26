
use clap::{App, Arg, SubCommand, AppSettings};

const EDIT_HELP: &str = "Edit register description files using text-based user interface (TUI).
Warning: Saving the file deletes comments from the file.";


/// Possibly quits the program.
pub fn parse_command_line_args() -> Config {
    let matches = App::new("Register Description Tools")
        .version("0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(SubCommand::with_name("validate")
            .about("Validates register description files.")
            .arg(Arg::with_name("input")
                .takes_value(true)
                .required(true)
                .help("Input file.")))
        .subcommand(SubCommand::with_name("edit")
            .about(EDIT_HELP)
            .arg(Arg::with_name("input")
                .takes_value(true)
                .required(true)
                .help("Input file.")))
        .subcommand(SubCommand::with_name("new")
            .about("Create new register description files using text-based user interface (TUI).")
            .arg(Arg::with_name("output")
                .takes_value(true)
                .required(true)
                .help("Output file.")))
        .subcommand(SubCommand::with_name("generate")
            .about("Generate code from register description file.")
            .arg(Arg::with_name("input")
                .takes_value(true)
                .required(true)
                .help("Input file."))
            .arg(Arg::with_name("output")
                .takes_value(true)
                .short("o")
                .help("Output file.")
                .required(true))
            .arg(Arg::with_name("language")
                .takes_value(true)
                .short("l")
                .possible_values(&["rust"])
                .default_value("rust")
                .help("Select programming language for code generation.")))
        .get_matches();

    match matches.subcommand() {
        ("validate", Some(sub_m)) => {
            let file = sub_m.value_of("input").unwrap().to_owned();
            Config::Validate { file }
        },
        ("edit", Some(sub_m)) => {
            let file = sub_m.value_of("input").unwrap().to_owned();
            Config::Edit { file }
        },
        ("new", Some(sub_m)) => {
            let file = sub_m.value_of("output").unwrap().to_owned();
            Config::New { file }
        },
        ("generate", Some(sub_m)) => {
            let input = sub_m.value_of("input").unwrap().to_owned();
            let output = sub_m.value_of("output").unwrap().to_owned();
            Config::Generate {
                input, output, language: Language::Rust,
            }
        },
        _ => unreachable!()
    }
}

pub enum Config {
    Validate {
        file: String,
    },
    Edit {
        file: String,
    },
    New {
        file: String,
    },
    Generate {
        input: String,
        output: String,
        language: Language,
    }
}

pub enum Language {
    Rust,
}
