use cuddld_macros::{Display, Error};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const COLOR_RED: &str = "\x1b[91m";
const COLOR_BLUE: &str = "\x1b[94m";
const COLOR_RESET: &str = "\x1b[0m";

#[macro_export]
macro_rules! assert_snapshot {
    ($expected:expr) => {{
        fn __path() {}

        let mut segments = std::any::type_name_of_val(&__path).split("::").collect::<Vec<_>>();
        assert_eq!(segments.pop(), Some("__path"));
        if let Some(last) = segments.last_mut() {
            if let Some(stripped) = last.strip_prefix("test_") {
                *last = stripped;
            }
        }

        let dest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("snapshots")
            .join(format!("{}.snap", segments.join("__")));

        $crate::snapshot::assert_snapshot(&dest, &$expected);
    }};
}

#[track_caller]
pub fn assert_snapshot(snapshot: &Path, expected: &str) {
    match run_diff(snapshot, expected) {
        Ok(()) => {}
        Err(DiffError::Mismatch(output)) => {
            if std::env::var_os("UPDATE_EXPECT").is_some() {
                if let Some(parent) = snapshot.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(&parent).expect("failed to create parent dir");
                    }
                }
                std::fs::write(snapshot, expected.as_bytes()).expect("faileed to write snapshot");
                return;
            }

            // cargo-nextest doesn't like when `diff` emits `\t`, it messes up with the layout.
            let output = output.replace('\t', "    ");

            eprintln!();
            eprintln!(
                "{COLOR_RED}Output doesn't match the expected snapshot: \
                 {COLOR_BLUE}(run with UPDATE_EXPECT=1 to fix)\
                 {COLOR_RESET}"
            );
            eprintln!("{output}");
            eprintln!();

            panic!("snapshots didn't match")
        }
        Err(err) => panic!("failed to compare snapshots: {err}"),
    }
}

fn run_diff(existing: &Path, content: &str) -> Result<(), DiffError> {
    let mut child = Command::new("diff")
        .args(["--unified", "--new-file", "--color=always"])
        .arg(existing)
        .arg("/proc/self/fd/0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(DiffError::Spawn)?;

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(content.as_bytes()).map_err(DiffError::Stdin)?;
    drop(stdin);

    let output = child.wait_with_output().map_err(DiffError::Spawn)?;
    if output.status.success() {
        Ok(())
    } else {
        let diff = String::from_utf8(output.stdout).map_err(|_| DiffError::NonUtf8)?;
        Err(DiffError::Mismatch(diff))
    }
}

#[derive(Debug, Error, Display)]
enum DiffError {
    #[display("failed to spawn `diff`")]
    Spawn(#[source] std::io::Error),
    #[display("failed to provide the diffing content via stdin")]
    Stdin(#[source] std::io::Error),
    #[display("`diff` returned non-UTF-8 output")]
    NonUtf8,
    #[display("the two contents are different")]
    Mismatch(String),
}
