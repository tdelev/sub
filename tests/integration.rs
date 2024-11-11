use assert_cmd::Command;

fn sub() -> Command {
    Command::cargo_bin("sub").unwrap()
}

struct ReplacementTest {
    pattern: &'static str,
    replacement: &'static str,
    input: String,
    args: Vec<String>,
}

impl ReplacementTest {
    pub fn new(pattern: &'static str, replacement: &'static str) -> Self {
        ReplacementTest {
            pattern,
            replacement,
            input: String::new(),
            args: vec![],
        }
    }

    pub fn arg(&mut self, argument: &str) -> &mut Self {
        self.args.push(argument.to_owned());
        self
    }

    pub fn for_input(&mut self, input: &str) -> &mut Self {
        self.input = input.into();
        self
    }

    pub fn expect_output(&mut self, output: &'static str) -> &mut Self {
        sub()
            .args(&self.args)
            .arg(&self.pattern)
            .arg(&self.replacement)
            .write_stdin(self.input.as_str())
            .assert()
            .success()
            .stdout(output);
        self
    }
}

#[test]
fn basic_replacement() {
    ReplacementTest::new("foo", "bar")
        .for_input("foo dummy foo\n")
        .expect_output("bar dummy bar\n");
}

#[test]
fn no_trailing_newline() {
    ReplacementTest::new("foo", "bar")
        .for_input("foo dummy foo\nfoo")
        .expect_output("bar dummy bar\nbar");
}

#[test]
fn windows_newline() {
    ReplacementTest::new("foo", "bar")
        .for_input("foo dummy foo\r\nfoo\r\n")
        .expect_output("bar dummy bar\r\nbar\r\n");
}

#[test]
fn case_insensitive_replacement() {
    ReplacementTest::new("Foo", "bar")
        .arg("-i")
        .for_input("foo dummy foo\r\nfoo\r\n")
        .expect_output("bar dummy bar\r\nbar\r\n");
}

#[test]
fn regex_replacement() {
    ReplacementTest::new(r"\w+", "x")
        .arg("-i")
        .for_input("foo dummy foo\r\nfoo\r\n")
        .expect_output("x x x\r\nx\r\n");
}
