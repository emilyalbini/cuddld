use crate::picohcl::ast::ResolvedExpression;
use crate::picohcl::deserialize::FromHclExpression;
use plinky_error::{ErasedContext as _, ErasedError, bail, erased};

pub trait HclFunction {
    fn call(&self, args: Vec<ResolvedExpression>) -> Result<ResolvedExpression, ErasedError>;
}

impl<T> HclFunction for T
where
    T: Fn(Vec<ResolvedExpression>) -> Result<ResolvedExpression, ErasedError>,
{
    fn call(&self, args: Vec<ResolvedExpression>) -> Result<ResolvedExpression, ErasedError> {
        (*self)(args)
    }
}

// Unfortunately it's not possible to define a trait implementation like this:
//
// ```rust
// impl<T, P1> HclFunction for T
// where
//     T: Fn(P1) -> ResolvedExpression,
//     P1: FromHclExpression,
// ```
//
// That's because the type system wants P1 to be present either in Self, the trait name, or the
// type name. In this case it is an error because in theory there could be multiple different Fn
// implementations for the trait.
//
// To work around that, we create an intermediate IntoHclFunction that is generic over all the
// params of the function, which returns a boxed HclFunction (by returning a closure). This
// allows for P1 to be present in the trait name and then erase it a TemplateFunction.
pub trait IntoHclFunction<T> {
    fn into_hcl_function(self) -> Box<dyn HclFunction>;
}

macro_rules! impl_template_function {
    ($($param:ident),*) => {
        impl<F, $($param),*> IntoHclFunction<($($param,)*)> for F
        where
            F: Fn($($param),*) -> Result<ResolvedExpression, ErasedError> + 'static,
            $($param: FromHclExpression),*
        {
            #[allow(non_snake_case)]
            fn into_hcl_function(self) -> Box<dyn HclFunction> {
                Box::new(move |params: Vec<ResolvedExpression>| -> Result<ResolvedExpression, ErasedError> {
                    let mut params = params.into_iter();
                    let mut position = 0;

                    $(
                        position += 1;
                        let $param = $param::from_expression(
                            params.next().
                            ok_or_else(|| erased!("too few arguments"))?
                        ).with_context(|| format!("invalid argument in position {position}"))?;
                    )*

                    if params.next().is_some() {
                        bail!("too many arguments");
                    }

                    self($($param),*)
                })
            }
        }
    }
}

// Implement TemplateFunction for multiple arguments.
impl_template_function!(P1);
impl_template_function!(P1, P2);
impl_template_function!(P1, P2, P3);
impl_template_function!(P1, P2, P3, P4);
impl_template_function!(P1, P2, P3, P4, P5);
impl_template_function!(P1, P2, P3, P4, P5, P6);
impl_template_function!(P1, P2, P3, P4, P5, P6, P7);
impl_template_function!(P1, P2, P3, P4, P5, P6, P7, P8);
