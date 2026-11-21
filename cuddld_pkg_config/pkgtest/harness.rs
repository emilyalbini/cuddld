use cuddld_error::ErasedError;
use cuddld_pkg_config::PkgConfig;
use cuddld_test_harness::nextest::Test;
use cuddld_test_harness::snapshot::assert_snapshot;
use cuddld_utils::posix_shell_quote;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

fn test(path: PathBuf) -> Result<(), ErasedError> {
    let rendered = match PkgConfig::parse(&read_to_string(&path)?) {
        Ok(parsed) => {
            let PkgConfig {
                name,
                description,
                url,
                version,
                requires,
                requires_private,
                conflicts,
                cflags,
                libs,
                libs_private,
            } = parsed;

            let mut output = "Successfully parsed the file!\n\n".to_string();

            let mut push_str = |name: &str, slot: Option<String>| {
                if let Some(value) = slot {
                    output.push_str(&format!("{name}: {value}\n"));
                }
            };
            push_str("Name", name);
            push_str("Description", description);
            push_str("URL", url);
            push_str("Version", version);
            push_str("Requires", requires);
            push_str("Requires.private", requires_private);
            push_str("Conflicts", conflicts);

            let mut push_args = |name: &str, slot: Option<Vec<String>>| {
                if let Some(value) = slot {
                    output.push_str(name);
                    output.push_str(": ");
                    let mut first = true;
                    for arg in value {
                        if first {
                            first = false;
                        } else {
                            output.push(' ');
                        }
                        output.push_str(&posix_shell_quote(&arg));
                    }
                    output.push('\n');
                }
            };
            push_args("CFlags", cflags);
            push_args("Libs", libs);
            push_args("Libs.private", libs_private);

            output
        }
        Err(err) => {
            format!("Failed to parse the file:\n\n{}", ErasedError::from(err).format_chain())
        }
    };

    let mut snap = path.clone();
    snap.set_extension("snap");
    assert_snapshot(&snap, &rendered);

    Ok(())
}

fn gather(path: &Path) -> Result<Vec<Test>, ErasedError> {
    let mut tests = Vec::new();
    for file in path.read_dir()? {
        let entry = file?.path();
        let name = entry.file_name().unwrap().to_str().unwrap().to_string();

        if entry.extension().and_then(|s| s.to_str()) == Some("pc") {
            tests.push(Test { name, ignored: false, exec: Box::new(move || test(entry)) });
        }
    }
    Ok(tests)
}

fn main() -> Result<(), ErasedError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("pkgtest");
    let tests = gather(&path)?;

    cuddld_test_harness::nextest::main(tests)
}
