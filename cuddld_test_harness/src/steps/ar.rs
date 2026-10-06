use crate::picohcl::ast::ResolvedExpression;
use crate::picohcl::{FromHcl, HclDeserializer};
use crate::utils::{file_name, run};
use crate::{Step, TestContext};
use cuddld_error::ErasedError;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub(crate) struct ArStep {
    output: PathBuf,
    content: Vec<PathBuf>,
    symbol_table: bool,
}

impl Step for ArStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let dest = ctx.dest.join(ctx.step_name);
        std::fs::create_dir_all(&dest)?;
        for input in &self.content {
            std::fs::copy(ctx.maybe_relative_to_src(input), dest.join(file_name(input)))?;
        }

        let mut flags = "rc".to_string();
        if self.symbol_table {
            flags.push('s');
        } else {
            flags.push('S');
        }

        run(Command::new("ar")
            .current_dir(&dest)
            .arg(flags)
            .arg(&self.output)
            .args(self.content.iter().map(file_name).collect::<Vec<_>>()))?;

        ctx.hcl.set_variable(ctx.step_name, ResolvedExpression::Path(dest.join(&self.output)));

        Ok(())
    }
}

impl FromHcl for ArStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self {
            output: de.field("output")?,
            content: de.field("content")?,
            symbol_table: de.opt_field("symbol-table")?.unwrap_or(true),
        })
    }
}
