use crate::Step;
use crate::picohcl::ast::ResolvedExpression;
use crate::picohcl::{FromHcl, HclDeserializer};
use crate::utils::file_name;
use cuddld_error::ErasedError;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct DirStep {
    files: Vec<PathBuf>,
}

impl Step for DirStep {
    fn run(&self, ctx: crate::TestContext<'_>) -> Result<(), ErasedError> {
        let dest = ctx.dest.join(ctx.step_name);
        std::fs::create_dir_all(&dest)?;

        for file in &self.files {
            std::fs::copy(ctx.maybe_relative_to_src(file), dest.join(file_name(file)))?;
        }

        ctx.hcl.set_variable(ctx.step_name, ResolvedExpression::Path(dest));

        Ok(())
    }
}

impl FromHcl for DirStep {
    fn from_hcl(de: &mut HclDeserializer) -> Result<Self, ErasedError> {
        Ok(Self { files: de.field("files")? })
    }
}
