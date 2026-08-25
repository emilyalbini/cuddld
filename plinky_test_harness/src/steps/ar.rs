use crate::gather::FromHcl;
use crate::picohcl::HclDeserializer;
use crate::template::{Template, Value};
use crate::utils::{file_name, run};
use crate::{Step, TestContext};
use plinky_error::ErasedError;
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ArStep {
    output: Template,
    content: Vec<Template>,
    #[serde(default = "default_true")]
    symbol_table: bool,
}

impl Step for ArStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let dest_name = self.output.resolve(ctx.template)?;
        let content =
            self.content.iter().map(|c| c.resolve(ctx.template)).collect::<Result<Vec<_>, _>>()?;

        let dest = ctx.dest.join(ctx.step_name);
        std::fs::create_dir_all(&dest)?;
        for input in &content {
            std::fs::copy(ctx.maybe_relative_to_src(&input), dest.join(file_name(input)))?;
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
            .arg(&dest_name)
            .args(content.iter().map(|c| file_name(c)).collect::<Vec<_>>()))?;

        ctx.template.set_variable(ctx.step_name, Value::Path(dest.join(dest_name)));

        Ok(())
    }

    fn templates(&self) -> Vec<Template> {
        std::iter::once(self.output.clone()).chain(self.content.iter().cloned()).collect()
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

fn default_true() -> bool {
    true
}
