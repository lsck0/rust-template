use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItemFn, spanned::Spanned};

pub fn expand(arguments: &TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !arguments.is_empty() {
        return Err(syn::Error::new(arguments.span(), "`#[timed]` takes no arguments"));
    }

    let mut function = syn::parse2::<ImplItemFn>(item)?;
    let name = function.sig.ident.to_string();
    let body = &function.block;

    function.block = syn::parse_quote! {{
        let _timer = ::telemetry::FunctionTimer::start(::std::concat!(::std::module_path!(), "::", #name));
        #body
    }};

    return Ok(quote! { #function });
}
