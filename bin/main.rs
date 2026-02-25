#![cfg_attr(flux, flux::ignore)]

use std::{env, process::ExitCode};

use {{crate_name}}::build_info;

const USAGE: &str = concat!("usage: ", env!("CARGO_PKG_NAME"), " [--version]");
const USAGE_ERROR_EXIT_CODE: u8 = 2;
const VERSION_FLAG: &str = "--version";
const TELEMETRY_ERROR_EXIT_CODE: u8 = 1;

#[macros::timed]
#[tracing::instrument]
fn greet() -> &'static str {
    tracing::info!("greeting");
    return "Hello, world!";
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();

    let telemetry = telemetry::init(telemetry::Service {
        name: env!("CARGO_PKG_NAME"),
        version: build_info::VERSION,
    });
    let _telemetry = match telemetry {
        Ok(telemetry) => telemetry,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(TELEMETRY_ERROR_EXIT_CODE);
        }
    };

    match arguments.as_slice() {
        [] => {
            tracing::info!(version = build_info::VERSION, commit = build_info::COMMIT, "started");
            println!("{}", greet());
            return ExitCode::SUCCESS;
        }
        [flag] if flag == VERSION_FLAG => {
            println!(
                "{} {} ({} {}, {}, {})",
                env!("CARGO_PKG_NAME"),
                build_info::VERSION,
                build_info::COMMIT,
                build_info::COMMIT_TIME,
                build_info::PROFILE,
                build_info::TARGET,
            );
            return ExitCode::SUCCESS;
        }
        _ => {
            tracing::error!(arguments = arguments.join(" "), "unexpected arguments");
            eprintln!("error: unexpected arguments: {}\n{USAGE}", arguments.join(" "));
            return ExitCode::from(USAGE_ERROR_EXIT_CODE);
        }
    }
}
