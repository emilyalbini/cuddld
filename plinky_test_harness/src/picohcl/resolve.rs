use crate::picohcl::ast::{
    AssignmentStatement, BlockStatement, Expression, ResolvedExpression, ResolvedInterpolation,
    ResolvedInterpolationChunk, Statement,
};
use crate::picohcl::builtins::register_builtins;
use crate::picohcl::functions::{HclFunction, IntoHclFunction};
use plinky_error::{ErasedContext as _, ErasedError};
use plinky_macros::{Display, Error};
use std::collections::HashMap;

pub struct HclContext {
    variables: HashMap<String, ResolvedExpression>,
    functions: HashMap<String, Box<dyn HclFunction>>,
}

impl HclContext {
    pub(crate) fn builtin() -> Self {
        let mut ctx = Self { variables: HashMap::new(), functions: HashMap::new() };
        register_builtins(&mut ctx);
        ctx
    }

    pub fn set_variable(&mut self, name: &str, value: ResolvedExpression) {
        self.variables.insert(name.into(), value);
    }

    pub fn register_function<T>(
        &mut self,
        name: &str,
        function: impl IntoHclFunction<T> + 'static,
    ) {
        self.functions.insert(name.into(), function.into_hcl_function());
    }
}

#[derive(Clone)]
pub(crate) struct HclResolver {
    unresolved: Vec<Statement>,
    resolved: Vec<Statement<ResolvedExpression>>,
}

impl HclResolver {
    pub(crate) fn new(unresolved: Vec<Statement>) -> Self {
        Self { unresolved, resolved: Vec::new() }
    }

    pub(crate) fn resolved(&self) -> Option<Vec<Statement<ResolvedExpression>>> {
        if self.unresolved.is_empty() { Some(self.resolved.clone()) } else { None }
    }

    pub(crate) fn try_resolve(&mut self, context: &HclContext) -> Result<(), HclResolveError> {
        let mut result = Ok(());
        let mut still_unresolved = Vec::new();
        for statement in self.unresolved.drain(..) {
            match resolve_statement(&statement, context) {
                Ok(resolved) => self.resolved.push(resolved),
                Err(HclResolveError::MissingVariable(_)) => still_unresolved.push(statement),
                Err(err) => {
                    still_unresolved.push(statement);
                    result = Err(err);
                }
            }
        }
        self.unresolved = still_unresolved;
        result
    }
}

fn resolve_statement(
    statement: &Statement,
    ctx: &HclContext,
) -> Result<Statement<ResolvedExpression>, HclResolveError> {
    match statement {
        Statement::Assignment(assignment) => Ok(Statement::Assignment(AssignmentStatement {
            key: assignment.key.clone(),
            value: resolve_expression(&assignment.value, ctx)?,
        })),
        Statement::Block(block) => {
            let mut contents = Vec::new();
            for stmt in &block.contents {
                contents.push(resolve_statement(stmt, ctx)?);
            }
            Ok(Statement::Block(BlockStatement {
                kind: block.kind.clone(),
                name: block.name.clone(),
                contents,
            }))
        }
    }
}

fn resolve_expression(
    expression: &Expression,
    ctx: &HclContext,
) -> Result<ResolvedExpression, HclResolveError> {
    Ok(match expression {
        Expression::String(s) => ResolvedExpression::String(s.clone()),
        Expression::Bool(b) => ResolvedExpression::Bool(*b),
        Expression::Path(p) => ResolvedExpression::Path(p.clone()),
        Expression::List(list) => {
            let mut result = Vec::new();
            for expr in list {
                result.push(resolve_expression(expr, ctx)?);
            }
            ResolvedExpression::List(result)
        }
        Expression::Variable(var) => {
            if let Some(content) = ctx.variables.get(&var.0) {
                content.clone()
            } else {
                return Err(HclResolveError::MissingVariable(var.0.clone()));
            }
        }
        Expression::Call(call) => {
            if let Some(function) = ctx.functions.get(&call.name) {
                let mut args = Vec::new();
                for arg in &call.args {
                    args.push(resolve_expression(arg, ctx)?);
                }
                function
                    .call(args)
                    .with_context(|| format!("failed to call function {}", call.name))
                    .map_err(HclResolveError::FunctionCall)?
            } else {
                return Err(HclResolveError::UnknownFunction(call.name.clone()));
            }
        }
        Expression::Interpolation(segments) => {
            let mut resolved = Vec::new();
            for expr in segments {
                match resolve_expression(expr, ctx)? {
                    ResolvedExpression::String(s) => {
                        resolved.push(ResolvedInterpolationChunk::String(s))
                    }
                    ResolvedExpression::Path(p) => {
                        resolved.push(ResolvedInterpolationChunk::Path(p))
                    }
                    ResolvedExpression::ResolvedInterpolation(ps) => {
                        resolved.extend(ps.0.into_iter())
                    }
                    ResolvedExpression::Bool(_) => return Err(HclResolveError::BoolIntoString),
                    ResolvedExpression::List(_) => return Err(HclResolveError::ListIntoString),
                }
            }
            if resolved.iter().all(|ps| matches!(ps, ResolvedInterpolationChunk::String(_))) {
                let mut strings = Vec::new();
                for item in resolved {
                    let ResolvedInterpolationChunk::String(s) = item else { unreachable!() };
                    strings.push(s);
                }
                ResolvedExpression::String(strings.join(""))
            } else {
                ResolvedExpression::ResolvedInterpolation(ResolvedInterpolation(resolved))
            }
        }
    })
}

#[derive(Debug, Display, Error)]
pub(crate) enum HclResolveError {
    #[display("missing variable {f0}")]
    MissingVariable(String),
    #[display("cannot convert bool into string")]
    BoolIntoString,
    #[display("cannot convert list into string")]
    ListIntoString,
    #[display("unknown function: {f0}")]
    UnknownFunction(String),
    #[display("{f0}")]
    FunctionCall(ErasedError),
}
