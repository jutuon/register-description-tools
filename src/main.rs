#![recursion_limit="128"]

pub mod config;
pub mod logic;
pub mod ui;

fn main() {
    let config = config::parse_command_line_args();

    logic::run(config)
}
