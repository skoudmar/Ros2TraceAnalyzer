use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Ident};

#[proc_macro_derive(TryFromBtFieldConst, attributes(bt2))]
pub fn derive_try_from_bt_field_const(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = match &input.data {
        Data::Struct(data_struct) => impl_from_bt_field_const(name, &data_struct.fields),
        _ => syn::Error::new_spanned(input, "Only structs are supported").to_compile_error(),
    };

    TokenStream::from(expanded)
}

fn impl_from_bt_field_const(name: &Ident, fields: &Fields) -> proc_macro2::TokenStream {
    let field_conversions = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_conversion = generate_field_conversion(field);
        quote! { #field_name: #field_conversion }
    });

    quote_spanned! {fields.span()=>
        impl TryFrom<bt2_sys::field::BtFieldConst> for #name {
            type Error = bt2_sys::field::ConversionError;

            fn try_from(bt_field: bt2_sys::field::BtFieldConst) -> Result<Self, Self::Error> {
                let bt_field = bt_field.try_into_struct()?;
                Ok(Self {
                    #(#field_conversions),*
                })
            }
        }
    }
}

// Generates the conversion code for each field, checking for a `try_from` attribute.
fn generate_field_conversion(field: &Field) -> proc_macro2::TokenStream {
    if field.ident.is_none() {
        return syn::Error::new_spanned(field, "Tuple structs are not supported")
            .to_compile_error();
    }
    let field_name = field.ident.as_ref().unwrap();
    let field_span = field.span();
    let field_name_str = format!(r#"c"{field_name}""#)
        .parse::<proc_macro2::TokenStream>()
        .unwrap();
    let try_from_attr = field.attrs.iter().try_fold(None, |acc, attr| {
        let parsed = parse_attribute(attr);
        if acc.is_some() {
            if !matches!(parsed, Ok(None)) {
                return Err(syn::Error::new_spanned(
                    attr,
                    "Multiple `bt2` attributes are not allowed",
                ));
            }
            return Ok(acc);
        }
        parsed
    });

    if let Err(e) = try_from_attr {
        return e.to_compile_error();
    }

    let conversion = match try_from_attr.unwrap() {
        Some(Conversion {
            try_from,
            is_non_zero: true,
        }) => match try_from {
            TryFromType::Bool => {
                let conversion = convert(TryFromType::Bool, field_name, field_span);
                quote_spanned! {field_span=>
                    {#conversion}.get_value()
                }
            }
            TryFromType::Int | TryFromType::Uint => {
                let conversion = convert(try_from, field_name, field_span);
                quote_spanned! {field_span=>
                    0 != {#conversion}.get_value()
                }
            }
            TryFromType::String => {
                syn::Error::new_spanned(field, "is_non_zero is not supported for strings")
                    .into_compile_error()
            }
            TryFromType::Array => {
                syn::Error::new_spanned(field, "is_non_zero is not supported for arrays")
                    .into_compile_error()
            }
        },
        Some(Conversion {
            try_from: TryFromType::Array,
            is_non_zero: false,
        }) => {
            let conversion = convert(TryFromType::Array, field_name, field_span);
            quote_spanned! {field_span=>
                {#conversion}.try_into()?
            }
        }
        Some(Conversion {
            try_from,
            is_non_zero: false,
        }) => {
            let conversion = convert(try_from, field_name, field_span);
            quote_spanned! {field_span=>
                {#conversion}.get_value().try_into()?
            }
        }
        None => default_field_conversion(field),
    };

    quote_spanned! {field_span=>
        {
            let Some(bt_field) = bt_field.get_field_by_name_cstr(#field_name_str) else {
                return Err(bt2_sys::field::StructConversionError::field_not_found(stringify!(#field_name)).into());
            };
            #conversion
        }
    }
}

fn convert(try_from: TryFromType, field_name: &Ident, span: Span) -> proc_macro2::TokenStream {
    match try_from {
        TryFromType::Bool => {
            quote_spanned! {span=> bt_field.try_into_bool().map_err(|e| bt2_sys::field::StructConversionError::field_conversion_error(stringify!(#field_name), e))? }
        }
        TryFromType::Int => {
            quote_spanned! {span=> bt_field.try_into_int().map_err(|e| bt2_sys::field::StructConversionError::field_conversion_error(stringify!(#field_name), e))? }
        }
        TryFromType::Uint => {
            quote_spanned! {span=> bt_field.try_into_uint().map_err(|e| bt2_sys::field::StructConversionError::field_conversion_error(stringify!(#field_name), e))? }
        }
        TryFromType::String => {
            quote_spanned! {span=> bt_field.try_into_string().map_err(|e| bt2_sys::field::StructConversionError::field_conversion_error(stringify!(#field_name), e))? }
        }
        TryFromType::Array => {
            quote_spanned! {span=> bt_field.try_into_array().map_err(|e| bt2_sys::field::StructConversionError::field_conversion_error(stringify!(#field_name), e))? }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Conversion {
    try_from: TryFromType,
    is_non_zero: bool,
}

#[derive(Debug, Clone, Copy)]
enum TryFromType {
    Bool,
    Int,
    Uint,
    String,
    Array,
}

impl FromStr for TryFromType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bool" => Ok(Self::Bool),
            "i64" => Ok(Self::Int),
            "u64" => Ok(Self::Uint),
            "String" => Ok(Self::String),
            "array" => Ok(Self::Array),
            _ => Err(()),
        }
    }
}

fn parse_attribute(attr: &syn::Attribute) -> syn::Result<Option<Conversion>> {
    if attr.path().is_ident("bt2") {
        let mut try_from = None;
        let mut is_non_zero = false;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("try_from") {
                let expr: syn::Expr = meta.value()?.parse()?;
                let ty = quote! { #expr }.to_string();

                ty.parse().map_or_else(
                    |()| Err(meta.error(format!("unknown bt2 type {ty:?}"))),
                    |ty| {
                        try_from = Some(ty);
                        Ok(())
                    },
                )
            } else if meta.path.is_ident("is_non_zero") {
                is_non_zero = true;
                Ok(())
            } else {
                Err(meta.error("unknown attribute"))
            }
        })?;

        if is_non_zero && try_from.is_none() {
            Err(syn::Error::new_spanned(
                attr,
                "The `is_non_zero` attribute requires a `try_from` attribute to determine the field type",
            ))
        } else {
            Ok(Some(Conversion {
                try_from: try_from.unwrap_or(TryFromType::String),
                is_non_zero,
            }))
        }
    } else {
        Ok(None)
    }
}

// Provides default conversion based on the field type when no `try_from` attribute is given.
fn default_field_conversion(field: &Field) -> proc_macro2::TokenStream {
    let Some(field_name) = &field.ident else {
        return syn::Error::new_spanned(field, "Tuple structs are not supported")
            .to_compile_error();
    };
    let field_span = field.span();

    quote_spanned! {field_span=>
        bt_field.try_into().map_err(|e| bt2_sys::field::StructConversionError::field_conversion_error(stringify!(#field_name), e))?
    }
}
