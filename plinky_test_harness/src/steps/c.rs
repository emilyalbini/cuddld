use crate::gather::FromHcl;
use crate::picohcl::{FromHclString, HclDeserializer};
use crate::template::{Template, Value};
use crate::utils::{file_name, run};
use crate::{Arch, Step, TestContext};
use plinky_error::{ErasedError, bail};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct CStep {
    source: Template,
    output: Option<Template>,
    libc: Libc,
    relocation: Relocation,
}

impl Step for CStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let source = ctx.maybe_relative_to_src(self.source.resolve(&*ctx.template)?);
        let source_name = file_name(&source);
        let dest_name = file_name(&source.with_extension("o"));

        let dest = ctx.dest.join(ctx.step_name);
        std::fs::create_dir_all(&dest)?;
        std::fs::copy(&source, dest.join(&source_name))?;

        run(Command::new("cc")
            .current_dir(&dest)
            .arg("-c")
            // Disable control-flow protection, as it's not implemented in the linker right now.
            .arg("-fcf-protection=none")
            .arg("-o")
            .arg(&dest_name)
            .arg(match ctx.arch {
                Arch::X86 => "-m32",
                Arch::X86_64 => "-m64",
            })
            .args(match self.libc {
                Libc::Freestanding => &["-nostdlib"],
            })
            .args(match self.relocation {
                Relocation::Static => &["-fno-pic"] as &[&str],
                Relocation::PicOnlyGot => &["-fPIC", "-fno-plt"],
                Relocation::Pic => &["-fPIC"],
            })
            .arg(&source_name))?;

        ctx.template.set_variable(ctx.step_name, Value::Path(dest.join(dest_name)));

        Ok(())
    }

    fn templates(&self) -> Vec<Template> {
        std::iter::once(self.source.clone()).chain(self.output.clone()).collect()
    }
}

impl FromHcl for CStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self {
            source: de.field("source")?,
            output: de.opt_field("output")?,
            libc: de.opt_field("libc")?.unwrap_or(Libc::Freestanding),
            relocation: de.field("relocation")?,
        })
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
enum Libc {
    Freestanding,
}

impl FromHclString for Libc {
    fn from_string(input: String) -> Result<Self, ErasedError> {
        match input.as_str() {
            "freestanding" => Ok(Libc::Freestanding),
            _ => bail!("unknown libc: {input}"),
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
enum Relocation {
    Static,
    PicOnlyGot,
    Pic,
}

impl FromHclString for Relocation {
    fn from_string(input: String) -> Result<Self, ErasedError> {
        match input.as_str() {
            "static" => Ok(Relocation::Static),
            "pic-only-got" => Ok(Relocation::PicOnlyGot),
            "pic" => Ok(Relocation::Pic),
            _ => bail!("unknown relocation: {input}"),
        }
    }
}
