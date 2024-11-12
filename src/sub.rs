use core::fmt;
use regex::RegexBuilder;
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};

use crate::Cli;

pub struct Sub<'a> {
    pub pattern: &'a str,
    pub replacement: &'a str,
    pub in_place: bool,
    pub whole_word: bool,
    pub ignore_case: bool,
    pub match_pattern: Option<&'a str>,
    pub inputs: Vec<Input<'a>>,
}

#[derive(Debug)]
pub enum SubError {
    FailedToWrite,
    InvalidUTF8,
    RegexError,
    FileNotFoundError(OsString),
    CanNotCreateTempFile,
    CanNotReadPermissions(OsString),
    CanNotSetPermissions(OsString),
    CanNotReplaceInPlace(OsString, io::Error),
}

#[derive(Debug, Clone)]
pub enum Input<'a> {
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
            CanNotCreateTempFile => write!(f, "Can not create temp file"),
            CanNotReadPermissions(path) => write!(
                f,
                "Can not read permissions on file '{}'",
                path.to_string_lossy()
            ),
            CanNotSetPermissions(path) => write!(
                f,
                "Can not set permissions on file '{}'",
                path.to_string_lossy()
            ),
            CanNotReplaceInPlace(path, error) => write!(
                f,
                "Can not replace in place for file '{}' with error '{}'",
                path.to_string_lossy(),
                error
            ),
        }
    }
}

type Result<T> = std::result::Result<T, SubError>;

impl<'a> Sub<'a> {
    pub fn init(cli: &'a Cli, inputs: Vec<Input<'a>>) -> Sub<'a> {
        Sub {
            pattern: &cli.pattern,
            replacement: &cli.replacement,
            in_place: cli.in_place,
            whole_word: cli.whole_word,
            ignore_case: cli.ignore_case,
            match_pattern: cli.line_match.as_deref(),
            inputs,
        }
    }

    pub fn run(&self, is_tty: bool) -> Result<()> {
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
                if let Input::File(path) = input {
                    let mut reader = create_reader(input, &stdin)?;
                    if self.in_place {
                        let temp_file = tempfile::Builder::new()
                            .prefix("sub_")
                            .tempfile()
                            .map_err(|_| SubError::CanNotCreateTempFile)?;

                        let mut writer = io::BufWriter::new(&temp_file);
                        self.replace(&re, &line_match_pattern, &mut reader, &mut writer)?;

                        let current_file_permissions = fs::metadata(path)
                            .map_err(|_| SubError::CanNotReadPermissions(path.to_os_string()))?
                            .permissions();

                        fs::set_permissions(temp_file.path(), current_file_permissions)
                            .map_err(|_| SubError::CanNotSetPermissions(temp_file.path().into()))?;

                        fs::copy(temp_file.path(), &path)
                            .map_err(|e| SubError::CanNotReplaceInPlace(path.to_os_string(), e))?;
                    } else {
                        unreachable!();
                    }
                } else {
                    let mut reader = create_reader(input, &stdin)?;
                    let mut writer = io::BufWriter::new(stdout.lock());
                    self.replace(&re, &line_match_pattern, &mut reader, &mut writer)?;
                }
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
