use core::fmt;
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process;

use clap::Parser;
use regex::RegexBuilder;

#[derive(Parser)]
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

struct Sub<'a> {
    pattern: &'a str,
    replacement: &'a str,
    in_place: bool,
    whole_word: bool,
    ignore_case: bool,
    match_pattern: Option<&'a str>,
    inputs: Vec<Input<'a>>,
}

#[derive(Debug, Clone)]
enum SubError {
    FailedToWrite,
    InvalidUTF8,
    RegexError,
    FileNotFoundError(OsString),
}

#[derive(Debug, Clone)]
enum Input<'a> {
    StdIn,
    File(&'a OsStr),
}

impl fmt::Display for SubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SubError::*;

        match self {
            FailedToWrite => write!(f, "Output stream has been closed"),
            InvalidUTF8 => write!(f, "Input contains invalid UTF-8"),
            RegexError => write!(f, "Regex error"),
            FileNotFoundError(path) => write!(f, "Can not open file '{}'", path.to_string_lossy()),
        }
    }
}

type Result<T> = std::result::Result<T, SubError>;

impl<'a> Sub<'a> {
    fn run(self, is_tty: bool) -> Result<()> {
        let pattern = if self.whole_word {
            format!(r"\b{}\b", self.pattern)
        } else {
            self.pattern.to_string()
        };
        let stdin = io::stdin();
        let stdout = io::stdout();
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(self.ignore_case)
            .build()
            .map_err(|_| SubError::RegexError)?;

        let line_match_pattern = self
            .match_pattern
            .map(|p| {
                RegexBuilder::new(&p)
                    .case_insensitive(self.ignore_case)
                    .build()
                    .map_err(|_| SubError::RegexError)
            })
            .transpose()?;
        for input in self.inputs.iter() {
            if is_tty {
                let mut reader = create_reader(input, &stdin)?;
                let mut output = stdout.lock();
                self.replace(&re, &line_match_pattern, &mut reader, &mut output)?;
            } else {
                let mut reader = create_reader(input, &stdin)?;
                let mut writer = io::BufWriter::new(stdout.lock());
                self.replace(&re, &line_match_pattern, &mut reader, &mut writer)?;
            }
        }

        Ok(())
    }

    fn replace(
        &self,
        re: &regex::Regex,
        line_match_pattern: &Option<regex::Regex>,
        reader: &mut dyn BufRead,
        writer: &mut dyn Write,
    ) -> Result<()> {
        let mut line_buffer = String::new();
        loop {
            line_buffer.clear();
            let num_bytes = reader
                .read_line(&mut line_buffer)
                .map_err(|_| SubError::InvalidUTF8)?;
            if num_bytes == 0 {
                break;
            }
            let new_line = if line_match_pattern
                .as_ref()
                .map_or(true, |m| m.is_match(&line_buffer))
            {
                re.replace_all(&line_buffer, self.replacement)
            } else {
                Cow::from(&line_buffer)
            };
            write!(writer, "{}", new_line).map_err(|_| SubError::FailedToWrite)?;
        }
        Ok(())
    }
}

fn create_reader(input: &Input<'_>, stdin: &io::Stdin) -> Result<Box<dyn BufRead>> {
    let reader: Box<dyn BufRead> = match input {
        Input::StdIn => Box::new(stdin.lock()),
        Input::File(path) => {
            let f = File::open(path).map_err(|_| SubError::FileNotFoundError(path.into()))?;
            Box::new(BufReader::new(f))
        }
    };
    Ok(reader)
}

fn main() {
    let cli = Cli::parse();
    let inputs = if cli.files.is_empty() {
        vec![Input::StdIn]
    } else {
        cli.files.iter().map(|f| Input::File(f)).collect()
    };
    let sub = Sub {
        pattern: &cli.pattern,
        replacement: &cli.replacement,
        in_place: cli.in_place,
        whole_word: cli.whole_word,
        ignore_case: cli.ignore_case,
        match_pattern: cli.line_match.as_deref(),
        inputs,
    };
    let result = sub.run(atty::is(atty::Stream::Stdout));
    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("[sub error]: {}", e);
            process::exit(1);
        }
    }
}
