use crate::picohcl::ast::ResolvedExpression;
use crate::picohcl::{FromHcl, HclDeserializer};
use crate::utils::{file_name, run};
use crate::{Arch, Step, TestContext};
use cuddld_error::ErasedError;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub(crate) struct LdStep {
    output: String,
    content: Vec<PathBuf>,
    extra_args: Vec<String>,
    shared_library: bool,
}

impl Step for LdStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let dest = ctx.dest.join(ctx.step_name);
        std::fs::create_dir_all(&dest)?;
        for input in &self.content {
            std::fs::copy(ctx.maybe_relative_to_src(&input), dest.join(file_name(input)))?;
        }

        run(Command::new("ld")
            .current_dir(&dest)
            .arg("-o")
            .arg(&self.output)
            .args(self.content.iter().map(|c| file_name(c)).collect::<Vec<_>>())
            .args(if self.shared_library { &["-shared"] as &[_] } else { &[] })
            .arg("--hash-style=both")
            .args(match ctx.arch {
                Arch::X86 => ["-m", "elf_i386"],
                Arch::X86_64 => ["-m", "elf_x86_64"],
            })
            .args(&self.extra_args))?;

        ctx.hcl.set_variable(ctx.step_name, ResolvedExpression::Path(dest.join(&self.output)));

        Ok(())
    }
}

impl FromHcl for LdStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self {
            output: de.field("output")?,
            content: de.field("content")?,
            extra_args: de.opt_field("extra-args")?.unwrap_or_else(Vec::new),
            shared_library: de.opt_field("shared-library")?.unwrap_or(false),
        })
    }
}
