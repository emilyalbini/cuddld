use crate::picohcl::ast::ResolvedExpression;
use crate::picohcl::{FromHcl, HclDeserializer};
use crate::{Step, TestContext};
use plinky_error::ErasedError;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct RenameStep {
    from: PathBuf,
    to: String,
}

impl Step for RenameStep {
    fn run(&self, ctx: TestContext<'_>) -> Result<(), ErasedError> {
        let from = ctx.maybe_relative_to_src(&self.from);

        let dest = ctx.dest.join(ctx.step_name).join(&self.to);
        std::fs::create_dir_all(dest.parent().unwrap())?;
        std::fs::copy(from, &dest)?;

        ctx.hcl.set_variable(ctx.step_name, ResolvedExpression::Path(dest));

        Ok(())
    }
}

impl FromHcl for RenameStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(RenameStep { from: de.field("from")?, to: de.field("to")? })
    }
}
