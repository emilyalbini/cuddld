use crate::error::Error;
use crate::parser::{Ident, Item, Parser, Struct, StructFields, Type};
use crate::utils::generate_impl_for;
use cuddld_macros_quote::quote;
use proc_macro::TokenStream;

pub(crate) fn derive(tokens: TokenStream) -> Result<TokenStream, Error> {
    let parsed = Parser::new(tokens).parse_struct()?;

    let fields32 = prepare_field_list(&parsed, true)?;
    let fields64 = prepare_field_list(&parsed, false)?;

    let raw_type = generate_impl_for(
        &Item::Struct(parsed.clone()),
        Some("cuddld_utils::raw_types::RawType"),
        quote! {
            #{ fn_read(&fields32, &fields64) }
            #{ fn_write(&fields32, &fields64) }
        },
    );

    let sized_raw_type = generate_impl_for(
        &Item::Struct(parsed.clone()),
        Some("cuddld_utils::raw_types::SizedRawType"),
        quote! {
            #{ fn_size(&fields32) }
        },
    );

    Ok(quote! {
        #raw_type
        #sized_raw_type
    })
}

fn fn_size(fields: &[Field<'_>]) -> TokenStream {
    let mut addends = Vec::new();
    for Field { field_ty, ctx, .. } in fields {
        let ctx = match ctx {
            Ctx::Default => quote!(ctx),
            Ctx::PointerSize => quote!(&cuddld_utils::raw_types::PointerSize(ctx)),
        };
        addends.push(quote! { + <#field_ty as cuddld_utils::raw_types::SizedRawType<_>>::size(#ctx) });
    }

    quote! {
        fn size(ctx: &cuddld_utils::raw_types::RawTypeContext) -> usize {
            0 #addends
        }
    }
}

fn fn_read(fields32: &[Field<'_>], fields64: &[Field<'_>]) -> TokenStream {
    fn render(fields: &[Field<'_>]) -> TokenStream {
        let mut setters = Vec::new();
        for Field { name, field_ty, ctx } in fields {
            let ctx = match ctx {
                Ctx::Default => quote!(ctx),
                Ctx::PointerSize => quote!(&cuddld_utils::raw_types::PointerSize(ctx)),
            };
            setters.push(quote! {
                #name: cuddld_utils::raw_types::RawReadError::wrap_field::<Self, _>(
                    stringify!(#name),
                    <#field_ty as cuddld_utils::raw_types::RawType<_>>::read(#ctx, reader)
                )?,
            });
        }
        quote! {
            Ok(Self { #setters })
        }
    }

    quote! {
        fn read(
            ctx: &cuddld_utils::raw_types::RawTypeContext,
            reader: &mut dyn std::io::Read,
        ) -> Result<Self, cuddld_utils::raw_types::RawReadError> {
            match ctx.bits {
                cuddld_utils::Bits::Bits32 => #{ render(fields32) },
                cuddld_utils::Bits::Bits64 => #{ render(fields64) },
            }
        }
    }
}

fn fn_write(fields32: &[Field<'_>], fields64: &[Field<'_>]) -> TokenStream {
    fn render(fields: &[Field<'_>]) -> TokenStream {
        let mut writes = Vec::new();
        for Field { name, field_ty, ctx } in fields {
            let ctx = match ctx {
                Ctx::Default => quote!(ctx),
                Ctx::PointerSize => quote!(&cuddld_utils::raw_types::PointerSize(ctx)),
            };
            writes.push(quote! {
                cuddld_utils::raw_types::RawWriteError::wrap_field::<Self, _>(
                    stringify!(#name),
                    <#field_ty as cuddld_utils::raw_types::RawType<_>>::write(
                        &self.#name, #ctx, writer,
                    )
                )?;
            });
        }
        quote! { { #writes } }
    }

    quote! {
        fn write(
            &self,
            ctx: &cuddld_utils::raw_types::RawTypeContext,
            writer: &mut dyn std::io::Write,
        ) -> Result<(), cuddld_utils::raw_types::RawWriteError> {
            match ctx.bits {
                cuddld_utils::Bits::Bits32 => #{ render(fields32) },
                cuddld_utils::Bits::Bits64 => #{ render(fields64) },
            }
            Ok(())
        }
    }
}

fn prepare_field_list(parsed: &Struct, is_elf32: bool) -> Result<Vec<Field<'_>>, Error> {
    let mut fields: Vec<Field> = Vec::new();

    let parsed_fields = match &parsed.fields {
        StructFields::StructLike(struct_like) => struct_like,
        _ => return Err(Error::new("only struct-like fields are supported")),
    };

    for field in parsed_fields {
        let mut insert_at = fields.len();
        let mut ctx = Ctx::Default;

        if let Some(attr) = field.attrs.get("pointer_size")? {
            attr.must_be_empty()?;
            ctx = Ctx::PointerSize;
        }
        if let Some(attr) = field.attrs.get("placed_on_elf32_after")? {
            let after = attr.get_equals_to_str()?;
            if is_elf32 {
                insert_at = fields.iter().position(|i| i.name.name == after).ok_or_else(|| {
                    Error::new(format!("could not find field called {after}")).span(attr.span)
                })? + 1;
            }
        }
        if let Some(attr) = field.attrs.get("placed_on_elf64_after")? {
            let after = attr.get_equals_to_str()?;
            if !is_elf32 {
                insert_at = fields.iter().position(|i| i.name.name == after).ok_or_else(|| {
                    Error::new(format!("could not find field called {after}")).span(attr.span)
                })? + 1;
            }
        }

        fields.insert(insert_at, Field { name: &field.name, field_ty: &field.ty, ctx });
    }

    Ok(fields)
}

struct Field<'a> {
    name: &'a Ident,
    field_ty: &'a Type,
    ctx: Ctx,
}

enum Ctx {
    Default,
    PointerSize,
}
