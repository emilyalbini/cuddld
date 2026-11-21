use cuddld_error::{ErasedError, bail};
use cuddld_utils::cli_lexer::{CliLexer, CliToken};

pub struct Test {
    pub name: String,
    pub ignored: bool,
    pub exec: Box<dyn FnOnce() -> Result<(), ErasedError>>,
}

pub fn main(tests: Vec<Test>) -> Result<(), ErasedError> {
    if std::env::var_os("NEXTEST").is_none() {
        bail!("the test suite can only be run through nextest");
    }

    let args = std::env::args().skip(1).collect::<Vec<_>>();

    let mut mode = SetOnce::new();
    let mut ignored = SetOnce::new();
    let mut test_name = SetOnce::new();

    let mut lexer = CliLexer::new(&args, &[]);
    while let Some(token) = lexer.next() {
        match &token {
            CliToken::LongFlag("exact") => mode.set(&token, CliMode::ExecuteExact)?,
            CliToken::LongFlag("list") => mode.set(&token, CliMode::List)?,
            CliToken::LongFlag("format") => {
                let format = lexer.expect_flag_value(&token)?;
                match format {
                    "terse" => {}
                    _ => bail!("unsupported --format={format}"),
                }
            }
            CliToken::LongFlag("ignored") => ignored.set(&token, true)?,
            CliToken::LongFlag("nocapture") => {}
            CliToken::StandaloneValue(value) => test_name.set(&token, value.to_string())?,
            _ => bail!("unexpected CLI argument: {token}"),
        }
    }

    match mode.0 {
        Some(CliMode::ExecuteExact) => {
            let Some(test_name) = test_name.0 else {
                bail!("missing test name");
            };
            for test in tests {
                if test.name == test_name {
                    match (test.exec)() {
                        Ok(()) => std::process::exit(0),
                        Err(err) => {
                            eprintln!("{}", err.format_chain());
                            std::process::exit(1);
                        }
                    }
                }
            }
            Ok(())
        }
        Some(CliMode::List) => {
            let ignored = ignored.0.unwrap_or(false);
            for test in tests {
                if !ignored || test.ignored {
                    println!("{}: test", test.name);
                }
            }
            std::process::exit(0);
        }
        None => bail!("no execution mode was provided"),
    }
}

struct SetOnce<T>(Option<T>);

impl<T> SetOnce<T> {
    fn new() -> Self {
        Self(None)
    }

    fn set(&mut self, token: &CliToken<'_>, value: T) -> Result<(), ErasedError> {
        if self.0.is_some() {
            bail!("{token} was passed multiple times");
        }
        self.0 = Some(value);
        Ok(())
    }
}

enum CliMode {
    ExecuteExact,
    List,
}
