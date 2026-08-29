use crate::picohcl::{FromHcl, FromHclString, HclDeserializer};
use crate::utils::{file_name, run};
use crate::{Arch, Step, TestContext};
use cuddld_error::{ErasedError, bail};
use std::path::PathBuf;
use std::process::Command;
use crate::picohcl::ast::ResolvedExpression;

#[derive(Debug)]
pub(crate) struct RustStep {
    source: PathBuf,
    panic: Panic,
}

impl Step for RustStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let source = ctx.maybe_relative_to_src(&self.source);
        let source_name = file_name(&source);
        let dest_name = format!("lib{}", file_name(&source.with_extension("a")));

        let dest = ctx.dest.join(ctx.step_name);
        std::fs::create_dir_all(&dest)?;
        std::fs::copy(&source, dest.join(&source_name))?;

        run(Command::new("rustc")
            .current_dir(&dest)
            .arg("--target")
            .arg(match ctx.arch {
                Arch::X86 => "i686-unknown-linux-gnu",
                Arch::X86_64 => "x86_64-unknown-linux-gnu",
            })
            .arg("--crate-type=staticlib")
            .arg(match self.panic {
                Panic::Abort => "-Cpanic=abort",
            })
            .arg("-o")
            .arg(&dest_name)
            .arg(&source_name))?;

        ctx.hcl.set_variable(ctx.step_name, ResolvedExpression::Path(dest.join(dest_name)));

        Ok(())
    }
}

impl FromHcl for RustStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self { source: de.field("source")?, panic: de.opt_field("panic")?.unwrap_or_default() })
    }
}

#[derive(Debug, Default, Clone, Copy)]
enum Panic {
    #[default]
    Abort,
}

impl FromHclString for Panic {
    fn from_string(input: String) -> Result<Self, ErasedError> {
        match input.as_str() {
            "abort" => Ok(Panic::Abort),
            other => bail!("unknown panic strategy: {other}"),
        }
    }
}
