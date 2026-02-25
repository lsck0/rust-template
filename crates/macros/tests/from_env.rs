use std::collections::HashMap;

use macros::FromEnv;
use pretty_assertions::{assert_eq, assert_ne};

#[derive(Debug, PartialEq, Eq, FromEnv)]
#[from_env(prefix = "APP_")]
struct Settings {
    port: u16,
    #[from_env(default = "4")]
    worker_count: usize,
    #[from_env(name = "DATABASE_URL")]
    database_url: Option<String>,
}

fn lookup_in(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();
    return move |key| map.get(key).cloned();
}

#[test]
fn reads_prefixed_names_and_explicit_names() {
    let lookup = lookup_in(&[
        ("APP_PORT", "8080"),
        ("APP_WORKER_COUNT", "16"),
        ("DATABASE_URL", "postgres://db"),
    ]);
    let settings = Settings::from_lookup(lookup).expect("every variable is valid");
    assert_eq!(
        settings,
        Settings {
            port: 8080,
            worker_count: 16,
            database_url: Some("postgres://db".to_owned())
        },
    );
}

#[test]
fn falls_back_to_the_default_and_to_none() {
    let settings = Settings::from_lookup(lookup_in(&[("APP_PORT", "1")])).expect("only APP_PORT is required");
    assert_eq!(
        settings,
        Settings {
            port: 1,
            worker_count: 4,
            database_url: None
        }
    );
}

#[test]
fn names_a_missing_required_variable() {
    let error = Settings::from_lookup(lookup_in(&[])).expect_err("APP_PORT is required");
    assert_eq!(error, SettingsEnvError::Missing { variable: "APP_PORT" });
    assert_eq!(error.to_string(), "environment variable APP_PORT is not set");
}

#[test]
fn names_a_variable_that_does_not_parse() {
    let error = Settings::from_lookup(lookup_in(&[("APP_PORT", "http")])).expect_err("http is no port");
    assert_eq!(
        error,
        SettingsEnvError::Invalid {
            variable: "APP_PORT",
            value: "http".to_owned(),
            reason: "invalid digit found in string".to_owned(),
        },
    );
    assert_eq!(
        error.to_string(),
        "environment variable APP_PORT = \"http\" does not parse: invalid digit found in string",
    );
}

#[test]
fn from_env_reads_the_process_environment() {
    // PATH is set in every environment the tests run in; everything else here is optional
    #[derive(FromEnv)]
    struct Path {
        path: String,
    }

    assert_ne!(Path::from_env().expect("PATH is set").path, "");
}

#[test]
fn rejects_unsupported_inputs_at_compile_time() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
