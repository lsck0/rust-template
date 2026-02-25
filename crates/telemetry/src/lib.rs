//! Logs, traces, metrics and profiles for the Grafana stack, set up by one call to [`init`].
//!
//! | variable                      | effect                                                                    |
//! |-------------------------------|---------------------------------------------------------------------------|
//! | `RUST_LOG`                    | `tracing` filter directives for every output; `info` when unset           |
//! | `LOG_FORMAT`                  | `pretty` or `json`, overriding the build-profile default                  |
//! | `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP/HTTP base URL, e.g. Alloy at `http://localhost:4318`; unset: no OTLP |
//! | `OTEL_EXPORTER_OTLP_HEADERS`  | headers for every OTLP request, e.g. `Authorization=Basic ...`            |
//! | `OTEL_RESOURCE_ATTRIBUTES`    | extra resource attributes, e.g. `deployment.environment=dev`              |
//! | `PYROSCOPE_SERVER_ADDRESS`    | Pyroscope URL, e.g. `http://localhost:4040`; unset: no profiling          |
//! | `PYROSCOPE_BASIC_AUTH_USER`   | Pyroscope basic auth user, with `PYROSCOPE_BASIC_AUTH_PASSWORD`           |
//! | `PYROSCOPE_TENANT_ID`         | Pyroscope tenant, sent as `X-Scope-OrgID`                                 |
//!
//! | feature     | adds                                                                        |
//! |-------------|-----------------------------------------------------------------------------|
//! | `otlp`      | the OTLP/HTTP exporters for logs, traces and metrics, and process metrics   |
//! | `profiling` | the Pyroscope agent                                                         |
//! | `tls`       | `https://` endpoints for both, through rustls with the aws-lc crypto backend |
//!
//! ```no_run
//! let _telemetry = telemetry::init(telemetry::Service { name: "my-app", version: "1.2.3" }).expect("telemetry");
//! tracing::info!("started");
//! ```

#[cfg(feature = "otlp")]
mod otlp;
#[cfg(all(feature = "otlp", target_os = "linux"))]
mod process;
#[cfg(feature = "profiling")]
mod profiling;

use std::{
    fmt,
    str::FromStr,
    sync::LazyLock,
    time::{Duration, Instant},
};

use macros::FromEnv;
use opentelemetry::{
    KeyValue, global,
    metrics::{Histogram, Meter},
};
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    filter::ParseError,
    layer::SubscriberExt,
    util::{SubscriberInitExt, TryInitError},
};

const METER_SCOPE: &str = env!("CARGO_PKG_NAME");
const FUNCTION_DURATION_METRIC: &str = "function.duration";
const FUNCTION_NAME_ATTRIBUTE: &str = "code.function.name";
#[cfg(any(feature = "otlp", feature = "profiling"))]
const SERVICE_VERSION_ATTRIBUTE: &str = "service.version";

type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync>;

static FUNCTION_DURATION: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    return meter()
        .f64_histogram(FUNCTION_DURATION_METRIC)
        .with_unit("s")
        .with_description("Wall time of each call of a #[timed] function.")
        .build();
});

#[derive(Debug, Clone, Copy)]
pub struct Service {
    pub name: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

impl LogFormat {
    #[must_use]
    pub const fn for_build() -> Self {
        return if cfg!(debug_assertions) {
            Self::Pretty
        } else {
            Self::Json
        };
    }
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        return match text {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            _ => Err(format!("expected `pretty` or `json`, got `{text}`")),
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, FromEnv)]
pub struct Settings {
    #[from_env(name = "RUST_LOG", default = "info")]
    pub log_filter: String,
    #[from_env(name = "LOG_FORMAT")]
    pub log_format: Option<LogFormat>,
    #[from_env(name = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,
    #[from_env(name = "PYROSCOPE_SERVER_ADDRESS")]
    pub pyroscope_server_address: Option<String>,
    #[from_env(name = "PYROSCOPE_BASIC_AUTH_USER")]
    pub pyroscope_basic_auth_user: Option<String>,
    #[from_env(name = "PYROSCOPE_BASIC_AUTH_PASSWORD")]
    pub pyroscope_basic_auth_password: Option<String>,
    #[from_env(name = "PYROSCOPE_TENANT_ID")]
    pub pyroscope_tenant_id: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    Settings(SettingsEnvError),
    LogFilter(ParseError),
    #[cfg(feature = "otlp")]
    Exporter(opentelemetry_otlp::ExporterBuildError),
    #[cfg(feature = "profiling")]
    Profiler(pyroscope::PyroscopeError),
    Subscriber(TryInitError),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        return match self {
            Self::Settings(error) => write!(formatter, "telemetry settings: {error}"),
            Self::LogFilter(error) => write!(formatter, "RUST_LOG: {error}"),
            #[cfg(feature = "otlp")]
            Self::Exporter(error) => write!(formatter, "OTLP exporter: {error}"),
            #[cfg(feature = "profiling")]
            Self::Profiler(error) => write!(formatter, "pyroscope: {error}"),
            Self::Subscriber(error) => write!(formatter, "tracing subscriber: {error}"),
        };
    }
}

impl std::error::Error for Error {}

/// With profiling on, the drop takes up to 10 s: pyroscope-rs joins a timer thread that wakes only on 10 s boundaries.
#[must_use = "dropping the guard stops telemetry at once"]
#[derive(Debug, Default)]
pub struct Telemetry {
    // drops first: its last push needs the exporters alive
    #[cfg(feature = "profiling")]
    profiler: Option<profiling::Profiler>,
    #[cfg(feature = "otlp")]
    otlp: Option<otlp::Exporters>,
}

impl Telemetry {
    #[must_use]
    pub const fn exports_otlp(&self) -> bool {
        #[cfg(feature = "otlp")]
        return self.otlp.is_some();
        #[cfg(not(feature = "otlp"))]
        return false;
    }

    #[must_use]
    pub const fn profiles(&self) -> bool {
        #[cfg(feature = "profiling")]
        return self.profiler.is_some();
        #[cfg(not(feature = "profiling"))]
        return false;
    }
}

pub fn init(service: Service) -> Result<Telemetry, Error> {
    let settings = Settings::from_env().map_err(Error::Settings)?;
    return init_with(service, &settings);
}

/// The OTLP exporters still read their own `OTEL_*` variables; `otlp_endpoint` overrides `OTEL_EXPORTER_OTLP_ENDPOINT`.
pub fn init_with(service: Service, settings: &Settings) -> Result<Telemetry, Error> {
    #[cfg_attr(
        not(any(feature = "otlp", feature = "profiling")),
        expect(unused_mut, reason = "nothing to start without a feature")
    )]
    let mut telemetry = Telemetry::default();
    #[cfg(not(any(feature = "otlp", feature = "profiling")))]
    let _ = service;
    let mut layers: Vec<BoxedLayer> = Vec::new();

    let stdout_filter = EnvFilter::try_new(&settings.log_filter).map_err(Error::LogFilter)?;
    let stdout = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);
    layers.push(match settings.log_format.unwrap_or_else(LogFormat::for_build) {
        LogFormat::Pretty => stdout.pretty().with_filter(stdout_filter).boxed(),
        LogFormat::Json => stdout.json().with_filter(stdout_filter).boxed(),
    });

    #[cfg_attr(
        all(feature = "otlp", feature = "profiling"),
        expect(unused_mut, reason = "only pushed to with a feature off")
    )]
    let mut inert_variables: Vec<&str> = Vec::new();

    match &settings.otlp_endpoint {
        #[cfg(feature = "otlp")]
        Some(endpoint) => {
            let (exporters, otlp_layers) = otlp::Exporters::start(service, endpoint, &settings.log_filter)?;
            layers.extend(otlp_layers);
            telemetry.otlp = Some(exporters);
        }
        #[cfg(not(feature = "otlp"))]
        Some(_) => inert_variables.push("OTEL_EXPORTER_OTLP_ENDPOINT (feature `otlp`)"),
        None => {}
    }

    match &settings.pyroscope_server_address {
        #[cfg(feature = "profiling")]
        Some(address) => telemetry.profiler = Some(profiling::Profiler::start(service, address, settings)?),
        #[cfg(not(feature = "profiling"))]
        Some(_) => inert_variables.push("PYROSCOPE_SERVER_ADDRESS (feature `profiling`)"),
        None => {}
    }

    tracing_subscriber::registry()
        .with(layers)
        .try_init()
        .map_err(Error::Subscriber)?;

    for variable in inert_variables {
        tracing::warn!(variable, "set, but its cargo feature is off: no effect");
    }
    tracing::debug!(
        otlp = settings.otlp_endpoint.as_deref(),
        pyroscope = settings.pyroscope_server_address.as_deref(),
        "telemetry initialized",
    );
    return Ok(telemetry);
}

/// Instruments bind to the provider current at their creation, so create them after [`init`].
#[must_use]
pub fn meter() -> Meter {
    return global::meter(METER_SCOPE);
}

#[derive(Debug)]
pub struct FunctionTimer {
    function: &'static str,
    start: Instant,
}

impl FunctionTimer {
    #[must_use = "the call is measured until the timer drops"]
    pub fn start(function: &'static str) -> Self {
        return Self {
            function,
            start: Instant::now(),
        };
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        return self.start.elapsed();
    }
}

impl Drop for FunctionTimer {
    fn drop(&mut self) {
        FUNCTION_DURATION.record(
            self.elapsed().as_secs_f64(),
            &[KeyValue::new(FUNCTION_NAME_ATTRIBUTE, self.function)],
        );
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const SERVICE: Service = Service {
        name: "telemetry-test",
        version: "0.0.0",
    };

    fn settings_from(pairs: &[(&str, &str)]) -> Result<Settings, SettingsEnvError> {
        return Settings::from_lookup(|key| {
            return pairs
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).to_owned());
        });
    }

    #[test]
    fn log_format_parses_its_two_names_only() {
        assert_eq!("pretty".parse(), Ok(LogFormat::Pretty));
        assert_eq!("json".parse(), Ok(LogFormat::Json));
        assert!("JSON".parse::<LogFormat>().is_err());
    }

    #[test]
    fn settings_default_to_stdout_only() {
        let settings = settings_from(&[]).expect("no variable is required");
        assert_eq!(settings.log_filter, "info");
        assert_eq!(settings.log_format, None);
        assert_eq!(settings.otlp_endpoint, None);
        assert_eq!(settings.pyroscope_server_address, None);
    }

    #[test]
    fn settings_reject_an_unknown_log_format() {
        let error = settings_from(&[("LOG_FORMAT", "xml")]).expect_err("xml is no log format");
        assert!(
            matches!(
                error,
                SettingsEnvError::Invalid {
                    variable: "LOG_FORMAT",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn init_rejects_an_invalid_filter() {
        let settings = settings_from(&[("RUST_LOG", "info,[")]).expect("RUST_LOG is a plain string here");
        assert!(matches!(init_with(SERVICE, &settings), Err(Error::LogFilter(_))));
    }

    #[test]
    fn init_installs_the_subscriber_only_once() {
        let telemetry = init_with(SERVICE, &settings_from(&[]).expect("defaults")).expect("first install");
        assert!(!telemetry.exports_otlp());
        assert!(!telemetry.profiles());
        drop(FunctionTimer::start("telemetry::tests::noop"));

        let error = init_with(SERVICE, &settings_from(&[]).expect("defaults")).expect_err("already installed");
        assert!(matches!(error, Error::Subscriber(_)));
        assert!(error.to_string().starts_with("tracing subscriber: "), "{error}");
    }
}
