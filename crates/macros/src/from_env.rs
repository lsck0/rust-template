#![expect(
    clippy::needless_continue,
    clippy::option_if_let_else,
    reason = "lints inside the darling derive expansions"
)]

use darling::{FromDeriveInput, FromField, ast::Data, util::Ignored};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, GenericArgument, Ident, PathArguments, Type, Visibility};

#[derive(FromDeriveInput)]
#[darling(attributes(from_env), supports(struct_named))]
struct Input {
    ident: Ident,
    vis: Visibility,
    generics: syn::Generics,
    data: Data<Ignored, Field>,
    #[darling(default)]
    prefix: String,
}

#[derive(FromField)]
#[darling(attributes(from_env))]
struct Field {
    ident: Option<Ident>,
    ty: Type,
    name: Option<String>,
    default: Option<String>,
}

fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let Some(GenericArgument::Type(inner)) = arguments.args.first() else {
        return None;
    };
    return Some(inner);
}

pub fn expand(input: TokenStream) -> darling::Result<TokenStream> {
    let input = Input::from_derive_input(&syn::parse2::<DeriveInput>(input)?)?;
    let Data::Struct(fields) = input.data else {
        unreachable!("darling rejects every shape but a struct with named fields");
    };

    let name = &input.ident;
    let visibility = &input.vis;
    let error = format_ident!("{name}EnvError");
    let mut errors = darling::Error::accumulator();
    let mut assignments = Vec::new();

    for field in fields.fields {
        let ident = field.ident.expect("darling only passes named fields");
        let key = field
            .name
            .unwrap_or_else(|| format!("{}{}", input.prefix, ident.to_string().to_uppercase()));
        let inner = option_inner(&field.ty);

        if inner.is_some() && field.default.is_some() {
            errors.push(darling::Error::custom(
                "an `Option` field takes no `default`: unset already means `None`",
            ));
            continue;
        }

        let parse = |ty: &Type| {
            quote! {
                value.parse::<#ty>().map_err(|reason| #error::Invalid {
                    variable: #key,
                    value: value.clone(),
                    reason: ::std::string::ToString::to_string(&reason),
                })?
            }
        };

        let value = match (inner, field.default) {
            (Some(inner), _) => {
                let parse = parse(inner);
                quote! {
                    match lookup(#key) {
                        ::std::option::Option::Some(value) => ::std::option::Option::Some(#parse),
                        ::std::option::Option::None => ::std::option::Option::None,
                    }
                }
            }
            (None, Some(default)) => {
                let parse = parse(&field.ty);
                quote! {{
                    let value = lookup(#key).unwrap_or_else(|| ::std::string::String::from(#default));
                    #parse
                }}
            }
            (None, None) => {
                let parse = parse(&field.ty);
                quote! {{
                    let value = lookup(#key).ok_or(#error::Missing { variable: #key })?;
                    #parse
                }}
            }
        };

        assignments.push(quote! { #ident: #value });
    }

    errors.finish()?;

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    return Ok(quote! {
        #[derive(Debug, Clone, PartialEq, Eq)]
        #visibility enum #error {
            Missing {
                variable: &'static str,
            },
            Invalid {
                variable: &'static str,
                value: ::std::string::String,
                reason: ::std::string::String,
            },
        }

        impl ::std::fmt::Display for #error {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                return match self {
                    Self::Missing { variable } => {
                        ::std::write!(formatter, "environment variable {variable} is not set")
                    }
                    Self::Invalid { variable, value, reason } => {
                        ::std::write!(formatter, "environment variable {variable} = {value:?} does not parse: {reason}")
                    }
                };
            }
        }

        impl ::std::error::Error for #error {}

        impl #impl_generics #name #type_generics #where_clause {
            #visibility fn from_env() -> ::std::result::Result<Self, #error> {
                return Self::from_lookup(|key| ::std::env::var(key).ok());
            }

            #visibility fn from_lookup(
                lookup: impl ::std::ops::Fn(&str) -> ::std::option::Option<::std::string::String>,
            ) -> ::std::result::Result<Self, #error> {
                return ::std::result::Result::Ok(Self { #(#assignments),* });
            }
        }
    });
}
