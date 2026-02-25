//! `#[derive(FromEnv)]` and `#[timed]`.

mod from_env;
mod timed;

use proc_macro::TokenStream;

/// A field reads `prefix` plus its upper-cased name, or its own `name`. `Option` fields may be unset; any other field
/// without a `default` is required.
///
/// ```
/// use macros::FromEnv;
///
/// #[derive(Debug, FromEnv)]
/// #[from_env(prefix = "APP_")]
/// struct Settings {
///     #[from_env(default = "4")]
///     worker_count: usize,
///     #[from_env(name = "DATABASE_URL")]
///     database_url: Option<String>,
/// }
///
/// let settings = Settings::from_lookup(|key| (key == "APP_WORKER_COUNT").then(|| "8".to_owned())).expect("valid");
/// assert_eq!(settings.worker_count, 8);
/// assert_eq!(settings.database_url, None);
/// ```
///
/// ```
/// # use macros::FromEnv;
/// #[derive(Debug, FromEnv)]
/// struct Limits {
///     max_connections: u32,
/// }
///
/// let error = Limits::from_lookup(|_| None).expect_err("MAX_CONNECTIONS is required");
/// assert_eq!(error, LimitsEnvError::Missing { variable: "MAX_CONNECTIONS" });
/// assert_eq!(error.to_string(), "environment variable MAX_CONNECTIONS is not set");
/// ```
#[proc_macro_derive(FromEnv, attributes(from_env))]
pub fn derive_from_env(input: TokenStream) -> TokenStream {
    return from_env::expand(input.into())
        .unwrap_or_else(darling::Error::write_errors)
        .into();
}

/// Records each call's wall time in the `function.duration` histogram. A drop guard, so early returns, `?` and panics
/// are measured too.
///
/// ```
/// #[macros::timed]
/// fn parse(input: &str) -> Result<u32, std::num::ParseIntError> {
///     return input.parse();
/// }
///
/// assert_eq!(parse("7"), Ok(7));
/// ```
#[proc_macro_attribute]
pub fn timed(arguments: TokenStream, item: TokenStream) -> TokenStream {
    return timed::expand(&arguments.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into();
}
