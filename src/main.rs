use core::fmt;
use std::io::Write;
use std::io::{self, BufRead};
use std::process;

use clap::{crate_description, crate_name, crate_version, Arg, Command};

#[derive(Debug, Clone)]
enum SubError {
    FailedToWrite,
    InvalidUTF8,
}

impl fmt::Display for SubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SubError::*;

        match self {
            FailedToWrite => write!(f, "Output stream has been closed"),
            InvalidUTF8 => write!(f, "Input contains invalid UTF-8"),
        }
    }
}

type Result<T> = std::result::Result<T, SubError>;

fn run(pattern: &str, replacement: &str) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let input = stdin.lock();
    let mut output = stdout.lock();

    for line in input.lines() {
        let line = line.map_err(|_| SubError::InvalidUTF8)?;
        let new_line = line.replace(pattern, replacement);
        writeln!(output, "{}", new_line).map_err(|_| SubError::FailedToWrite)?;
    }

    Ok(())
}

fn main() {
    let app = Command::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .args([
            Arg::new("pattern")
                .required(true)
                .help("The search pattern that should be replaced"),
            Arg::new("replacement")
                .required(true)
                .help("The string that should be substituted in"),
        ]);
    let matches = app.get_matches();
    let pattern = matches.get_one::<String>("pattern").unwrap();
    let replacement = matches.get_one::<String>("replacement").unwrap();
    let result = run(pattern, replacement);
    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[sub error]: {}", e);
            process::exit(1);
        }
    }
}
