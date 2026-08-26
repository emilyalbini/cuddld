use plinky_error::{ErasedError, bail, erased};
use plinky_test_harness::picohcl::ast::{ResolvedInterpolation, ResolvedInterpolationChunk};
use plinky_test_harness::picohcl::{FromHcl, HclDeserializer};
use plinky_test_harness::utils::RunAndSnapshot;
use plinky_test_harness::{Step, TestContext};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
struct PlinkyStep {
    cmd: Vec<ResolvedInterpolation>,
    kind: String,
    debug_print: Vec<String>,
    link_env: BTreeMap<String, ResolvedInterpolation>,
    run_env: BTreeMap<String, ResolvedInterpolation>,
    auxiliary_files: Vec<PathBuf>,
}

impl Step for PlinkyStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let mut runner = ctx.run_and_snapshot();
        let (res, err) = match self.kind.as_str() {
            "link-fail" => {
                (!self.link(&ctx, &mut runner)?, "linking was supposed to fail but passed!")
            }
            "link-pass" => {
                (self.link(&ctx, &mut runner)?, "linking was supposed to pass but failed!")
            }
            "run-fail" => {
                (!self.run(&ctx, &mut runner)?, "running was supposed to fail but passed!")
            }
            "run-pass" => {
                (self.run(&ctx, &mut runner)?, "running was supposed to pass but failed!")
            }
            kind => bail!("unsupported test kind: {kind}"),
        };
        runner.persist();

        if !res {
            bail!("{err}");
        }
        Ok(())
    }

    fn is_leaf() -> bool {
        true
    }
}

impl FromHcl for PlinkyStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self {
            cmd: de.field("cmd")?,
            kind: de.field("kind")?,
            debug_print: de.opt_field("debug-print")?.unwrap_or_default(),
            link_env: de.opt_field("link-env")?.unwrap_or_default(),
            run_env: de.opt_field("run-env")?.unwrap_or_default(),
            auxiliary_files: de.opt_field("auxiliary-files")?.unwrap_or_default(),
        })
    }
}

impl PlinkyStep {
    fn link(
        &self,
        ctx: &TestContext<'_>,
        runner: &mut RunAndSnapshot,
    ) -> Result<bool, ErasedError> {
        let dest = ctx.dest.join(ctx.step_name);
        std::fs::create_dir_all(&dest)?;

        let cmd = self.cmd.iter().map(|ps| handle_interpolation(&dest, ps)).collect::<Vec<_>>();

        let mut command = Command::new(env!("CARGO_BIN_EXE_ld.plinky"));
        command.current_dir(&dest).args(cmd).env("RUST_BACKTRACE", "1");
        for debug_print in &self.debug_print {
            command.args(["--debug-print", debug_print]);
        }
        for (key, value) in &self.link_env {
            command.env(key, handle_interpolation(&dest, value));
        }
        for file in &self.auxiliary_files {
            let name = file.file_name().unwrap();
            std::fs::copy(file, dest.join(name))?;
        }

        // In NixOS, the default linker is just a stub that errors out (since you are not supposed
        // to use dynamicly linked programs built outside of Nix). We thus need to set the correct
        // linker for it, which is provided by flake.nix through the environment variable.
        let dynamic_linker_var = match &ctx.arch {
            plinky_test_harness::Arch::X86 => "PLINKY_TEST_DYNAMIC_LINKER_32",
            plinky_test_harness::Arch::X86_64 => "PLINKY_TEST_DYNAMIC_LINKER_64",
        };
        command.arg("--dynamic-linker").arg(
            std::env::var_os(dynamic_linker_var)
                .ok_or_else(|| erased!("missing environment variable {dynamic_linker_var}"))?,
        );

        runner.run("linking", &mut command)
    }

    fn run(&self, ctx: &TestContext<'_>, runner: &mut RunAndSnapshot) -> Result<bool, ErasedError> {
        if !self.link(ctx, runner)? {
            runner.note("error: could not execute the program due to linking failing");
            return Ok(false);
        }

        let dest = ctx.dest.join(ctx.step_name);

        let mut command = Command::new(dest.join("a.out"));
        command.current_dir(&dest);
        for (key, value) in &self.run_env {
            command.env(key, handle_interpolation(&dest, value));
        }

        runner.run("running", &mut command)
    }
}

// To ensure we have consistent output in snapshot tests we want to move all of the input files in
// the destination directory, and replace their path in the command line. That's why we process
// the interpolation of paths and strings here.
fn handle_interpolation(dest: &Path, value: &ResolvedInterpolation) -> OsString {
    let mut result = OsString::new();
    for chunk in &value.0 {
        match chunk {
            ResolvedInterpolationChunk::String(s) => result.push(s),
            ResolvedInterpolationChunk::Path(path) => {
                let name = path.file_name().expect("path without name");
                if !dest.join(name).exists() {
                    copy_recursive(&path, dest).expect("failed to copy source element");
                }
                result.push(name);
            }
        }
    }
    result
}

fn copy_recursive(from: &Path, dest_dir: &Path) -> Result<(), std::io::Error> {
    let from_meta = std::fs::metadata(from)?;
    let name = from.file_name().expect("missing name");
    if from_meta.is_symlink() {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "cannot copy symlinks"))
    } else if from_meta.is_file() {
        std::fs::copy(from, dest_dir.join(name))?;
        Ok(())
    } else {
        let new_dir = dest_dir.join(name);
        std::fs::create_dir_all(&new_dir)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &new_dir)?;
        }
        Ok(())
    }
}

fn main() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("linktest");
    plinky_test_harness::main(&path, |definer| {
        definer.define_builtins()?.define::<PlinkyStep>("plinky")
    });
}
