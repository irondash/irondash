use case::RenameRule;
use proc_macro_error::{proc_macro_error, Diagnostic, Level};
use syn::DeriveInput;

use quote::quote;

mod attributes;
mod case;
mod from;
mod try_into;

use from::*;
use try_into::*;

#[proc_macro_derive(IntoValue, attributes(irondash))]
#[proc_macro_error]
pub fn into_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    let name = ast.ident;
    let token_stream = match ast.data {
        syn::Data::Struct(s) => FromStruct::new(name.clone(), ast.attrs).process(s),
        syn::Data::Enum(e) => FromEnum::new(name.clone(), ast.attrs).process(e),
        syn::Data::Union(_) => {
            Diagnostic::spanned(
                name.span(),
                Level::Error,
                "derive(IntoValue) is not supported for unions".into(),
            )
            .abort();
        }
    };

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let tokens = quote! {
        #[automatically_derived]
        impl #impl_generics From<#name #ty_generics> for ::irondash_message_channel::Value #where_clause {
            fn from(__ns_value: #name #ty_generics) -> Self {
                use ::irondash_message_channel::derive_internal::IsNone;
                #token_stream
            }
        }
    };
    proc_macro::TokenStream::from(tokens)
}

#[proc_macro_derive(TryFromValue, attributes(irondash))]
#[proc_macro_error]
pub fn try_from_value(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    let name = ast.ident;
    let token_stream = match ast.data {
        syn::Data::Struct(s) => TryIntoStruct::new(name.clone(), ast.attrs).process(s),
        syn::Data::Enum(e) => TryIntoEnum::new(name.clone(), ast.attrs).process(e),
        syn::Data::Union(_) => {
            Diagnostic::spanned(
                name.span(),
                Level::Error,
                "derive(TryFromValue) is not supported for unions".into(),
            )
            .abort();
        }
    };

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let tokens = quote! {
        #[automatically_derived]
        impl #impl_generics core::convert::TryFrom<::irondash_message_channel::Value> for #name #ty_generics #where_clause {
            type Error = ::irondash_message_channel::TryFromError;
            fn try_from(__ns_value: ::irondash_message_channel::Value) -> Result<Self, Self::Error> {
                use ::irondash_message_channel::derive_internal::Assign;
                #token_stream
            }
        }
    };
    proc_macro::TokenStream::from(tokens)
}

pub(crate) fn rename_field(
    original: &str,
    rename_rule: &RenameRule,
    rename: &Option<String>,
) -> String {
    if let Some(rename) = rename {
        return rename.clone();
    }
    rename_rule.apply_to_field(original)
}

pub(crate) fn rename_variant(
    original: &str,
    rename_rule: &RenameRule,
    rename: &Option<String>,
) -> String {
    if let Some(rename) = rename {
        return rename.clone();
    }
    rename_rule.apply_to_variant(original)
}
