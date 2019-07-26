
pub mod config;
pub mod logic;

fn main() {
    let config = config::parse_command_line_args();

    logic::run(config)
}
