use core::panic;
use std::str::FromStr;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Ident, Type};

#[proc_macro_derive(FromBtFieldConst, attributes(bt2))]
pub fn from_bt_field_const_derive(input: TokenStream) -> TokenStream {
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

    quote! {
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
        return syn::Error::new_spanned(field, "Field must have an identifier").to_compile_error();
    }
    let field_name = field.ident.as_ref().unwrap();
    let field_name_str = format!(r#"c"{field_name}""#).parse::<proc_macro2::TokenStream>().unwrap();
    let try_from_attr = field.attrs.iter().find_map(parse_attribute);


    let conversion = match try_from_attr {
        Some(TryFromType::Bool) => quote! { bt_field.try_into_bool()?.get_value().try_into()? },
        Some(TryFromType::Int) => quote! { bt_field.try_into_int()?.get_value().try_into()? },
        Some(TryFromType::Uint) => quote! { bt_field.try_into_uint()?.get_value().try_into()? },
        Some(TryFromType::String) => quote! { bt_field.try_into_string()?.get_value().try_into()? },
        Some(TryFromType::Array) => quote! { bt_field.try_into_array()?.get_value().try_into()? },
        None => default_field_conversion(field),
    };

    quote!{
        {
            let Some(bt_field) = bt_field.get_field_by_name_cstr(#field_name_str) else {
                return Err(bt2_sys::field::StructConversionError::FieldNotFound(stringify!(#field_name)).into());
            };
            #conversion
        }
    }
}

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
            "bool" => Ok(TryFromType::Bool),
            "int" => Ok(TryFromType::Int),
            "uint" => Ok(TryFromType::Uint),
            "string" => Ok(TryFromType::String),
            "array" => Ok(TryFromType::Array),
            _ => Err(()),
        }
    }
}

fn parse_attribute(attr: &syn::Attribute) -> Option<TryFromType> {
    if attr.path().is_ident("bt2") {
        let mut typ = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("type") {
                let expr: syn::Expr = meta.value()?.parse()?;
                let ty = quote! { #expr }.to_string().parse();

                if let Ok(ty) = ty {
                    typ = Some(ty);
                    Ok(())
                } else {
                    Err(meta.error("unknown bt2 type"))
                }
            } else {
                Err(meta.error("unknown attribute"))
            }
        })
        .unwrap();

        typ
    } else {
        None
    }
}

// Provides default conversion based on the field type when no `try_from` attribute is given.
fn default_field_conversion(field: &Field) -> proc_macro2::TokenStream {
    let field_name = &field.ident;
    let field_type = match &field.ty {
        Type::Path(type_path) => type_path.path.segments.last().unwrap().ident.to_string(),
        _ => panic!("Unsupported field type"),
    };

    match field_type.as_str() {
        "bool" => quote! { bt_field.into_bool().get_value() },
        "u64" => quote! { bt_field.into_uint().get_value() },
        "i64" => quote! { bt_field.into_int().get_value() },
        "String" => quote! { bt_field.into_string().get_value().to_string() },
        "Vec" => quote! { bt_field.into_array().into_vec() },
        _ => syn::Error::new_spanned(field, "Unsupported field type. To convert from supported type use attribute `#[bt2(type = \"uint\")]`.").to_compile_error(),
    }
}

