mod sub;
use clap::Parser;
use std::ffi::OsString;
use std::process;
use sub::{Input, Sub};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(help = "The search pattern that should be replaced")]
    pattern: String,
    #[arg(help = "The replacement string for the search pattern")]
    replacement: String,
    #[arg(help = "Input file(s) to perform the substitution on")]
    files: Vec<OsString>,
    #[arg(short, long, help = "Use case-insensitive search")]
    ignore_case: bool,
    #[arg(short = 'p', long, requires = "files", help = "Edit files in place")]
    in_place: bool,
    #[arg(short, long, help = "Only match the pattern on whole words")]
    whole_word: bool,
    #[arg(
        short = 'm',
        long = "match",
        value_name = "pattern",
        help = "Only substitute on lines that match the pattern"
    )]
    line_match: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    let inputs = if cli.files.is_empty() {
        vec![Input::StdIn]
    } else {
        cli.files.iter().map(|f| Input::File(f)).collect()
    };
    let sub = Sub::init(&cli, inputs);
    let result = sub.run(atty::is(atty::Stream::Stdout));
    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[sub error]: {}", e);
            process::exit(1);
        }
    }
}
