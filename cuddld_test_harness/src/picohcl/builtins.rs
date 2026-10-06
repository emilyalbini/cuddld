use crate::picohcl::HclContext;
use crate::picohcl::ast::ResolvedExpression;
use cuddld_error::{ErasedError, erased};
use std::path::PathBuf;

pub(super) fn register_builtins(ctx: &mut HclContext) {
    ctx.register_function("dirname", dirname);
    ctx.register_function("to_path", to_path);
}

fn dirname(path: PathBuf) -> Result<ResolvedExpression, ErasedError> {
    let parent = path.parent().ok_or_else(|| erased!("path {} has no parent", path.display()))?;
    Ok(ResolvedExpression::Path(parent.into()))
}

fn to_path(string: String) -> Result<ResolvedExpression, ErasedError> {
    Ok(ResolvedExpression::Path(string.into()))
}
