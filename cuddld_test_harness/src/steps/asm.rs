use crate::picohcl::ast::ResolvedExpression;
use crate::picohcl::{FromHcl, FromHclString, HclDeserializer};
use crate::utils::{file_name, run};
use crate::{Arch, Step, TestContext};
use cuddld_error::{ErasedError, bail};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub(crate) struct AsmStep {
    source: PathBuf,
    arch: Option<Arch>,
    output: Option<String>,
    assembler: Assembler,
    auxiliary_files: Vec<PathBuf>,
    emit_x86_used: bool,
}

impl Step for AsmStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let source = ctx.maybe_relative_to_src(&self.source);
        let source_name = file_name(&source);

        let dest_name = match &self.output {
            Some(name) => name.clone(),
            None => file_name(source.with_extension("o")),
        };

        let dest = ctx.dest.join(ctx.step_name);
        std::fs::create_dir_all(&dest)?;
        std::fs::copy(&source, dest.join(&source_name))?;

        for auxiliary in &self.auxiliary_files {
            let auxiliary = ctx.maybe_relative_to_src(auxiliary);
            std::fs::copy(&auxiliary, dest.join(file_name(&auxiliary)))?;
        }

        match self.assembler {
            Assembler::Nasm => {
                run(Command::new("nasm")
                    .current_dir(&dest)
                    .arg("-f")
                    .arg(match self.arch.unwrap_or(ctx.arch) {
                        Arch::X86 => "elf32",
                        Arch::X86_64 => "elf64",
                    })
                    .arg("-o")
                    .arg(&dest_name)
                    .arg(&source_name))?;
            }
            Assembler::Gnu => {
                // We invoke the assembler through the C compiler rather than invoking `as` directly,
                // because some of the assembly files require the C preprocessor to be exeucted.
                run(Command::new("cc")
                    .current_dir(&dest)
                    .arg("-c")
                    .arg(match self.arch.unwrap_or(ctx.arch) {
                        Arch::X86 => "-m32",
                        Arch::X86_64 => "-m64",
                    })
                    .arg(format!(
                        "-Wa,-mx86-used-note={}",
                        if self.emit_x86_used { "yes" } else { "no" }
                    ))
                    .arg("-o")
                    .arg(&dest_name)
                    .arg(&source_name))?;
            }
        }

        ctx.hcl.set_variable(ctx.step_name, ResolvedExpression::Path(dest.join(dest_name)));

        Ok(())
    }
}

impl FromHcl for AsmStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self {
            source: de.field("source")?,
            arch: de.opt_field("arch")?,
            output: de.opt_field("output")?,
            assembler: de.opt_field("assembler")?.unwrap_or(Assembler::Gnu),
            auxiliary_files: de.opt_field("auxiliary-files")?.unwrap_or_else(Vec::new),
            emit_x86_used: de.opt_field("emit-x86-used")?.unwrap_or(true),
        })
    }
}

#[derive(Debug, Default)]
enum Assembler {
    Nasm,
    #[default]
    Gnu,
}

impl FromHclString for Assembler {
    fn from_string(input: String) -> Result<Self, ErasedError> {
        match input.as_str() {
            "gnu" => Ok(Assembler::Gnu),
            "nasm" => Ok(Assembler::Nasm),
            _ => bail!("unknown assembler: {input}"),
        }
    }
}
