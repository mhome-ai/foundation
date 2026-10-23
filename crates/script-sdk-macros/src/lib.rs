use proc_macro::TokenStream;
use quote::quote;
use syn::{ext::IdentExt, parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(ScriptInput)]
pub fn script_input(input: TokenStream) -> TokenStream {
    derive(parse_macro_input!(input as DeriveInput), true)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(ScriptOutput)]
pub fn script_output(input: TokenStream) -> TokenStream {
    derive(parse_macro_input!(input as DeriveInput), false)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn derive(input: DeriveInput, is_input: bool) -> syn::Result<proc_macro2::TokenStream> {
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            input.generics,
            "script roots cannot be generic",
        ));
    }
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "script roots must have named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "script roots must be structs",
            ))
        }
    };
    let identifiers: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
    let names: Vec<_> = identifiers
        .iter()
        .map(|id| id.unraw().to_string())
        .collect();
    for (id, name) in identifiers.iter().zip(&names) {
        if name.len() > 255 {
            return Err(syn::Error::new_spanned(
                id,
                "script field name exceeds 255 UTF-8 bytes",
            ));
        }
    }
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let name = &input.ident;
    let metadata = quote! {
        fn fields() -> ::std::vec::Vec<::mhome_script_sdk::Field> {
            ::std::vec![#(::mhome_script_sdk::Field {
                name: #names.into(),
                ty: <#types as ::mhome_script_sdk::ScriptField>::field_type(),
            }),*]
        }
    };
    Ok(if is_input {
        quote! {
            impl ::mhome_script_sdk::ScriptInput for #name {
                #metadata
                fn from_value(value: ::mhome_script_sdk::Value) -> ::std::result::Result<Self, ::mhome_script_sdk::ScriptError> {
                    let mut args = ::mhome_script_sdk::Arguments::new(value)?;
                    let input = Self { #(#identifiers: args.take::<#types>(#names)?),* };
                    args.finish()?;
                    Ok(input)
                }
            }
        }
    } else {
        quote! {
            impl ::mhome_script_sdk::ScriptOutput for #name {
                #metadata
                fn into_value(self) -> ::mhome_script_sdk::Value {
                    ::mhome_script_sdk::Value::Object(::std::collections::BTreeMap::from([
                        #((#names.into(), ::mhome_script_sdk::Value::from(self.#identifiers))),*
                    ]))
                }
            }
        }
    })
}
