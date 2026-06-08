mod cli;
mod commands;
mod output;

use std::env;
use std::process::ExitCode;

use cli::{CliError, run};
use output::print_help;

const EXIT_USAGE: u8 = 2;
const EXIT_RUNTIME: u8 = 3;

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            print_help();
            ExitCode::from(EXIT_USAGE)
        }
        Err(CliError::Runtime(message)) => {
            eprintln!("{message}");
            ExitCode::from(EXIT_RUNTIME)
        }
    }
}
