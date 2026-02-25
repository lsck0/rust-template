use std::{env, fmt};

use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource, logs::SdkLoggerProvider, metrics::SdkMeterProvider, propagation::TraceContextPropagator,
    trace::SdkTracerProvider,
};
use tracing_subscriber::{EnvFilter, Layer};

use crate::{BoxedLayer, Error, SERVICE_VERSION_ATTRIBUTE, Service};

// the Pyroscope agent pushes from its own thread, which the SDK does not silence
const TARGETS_OFF: [&str; 11] = [
    "Pyroscope",
    "pyroscope",
    "reqwest",
    "hyper",
    "hyper_util",
    "h2",
    "rustls",
    "rustls_platform_verifier",
    "opentelemetry",
    "opentelemetry_sdk",
    "opentelemetry_otlp",
];

pub struct Exporters {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
    logger_provider: SdkLoggerProvider,
    #[cfg(target_os = "linux")]
    _process_metrics: crate::process::Instruments,
}

impl fmt::Debug for Exporters {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        return formatter.debug_struct("Exporters").finish_non_exhaustive();
    }
}

fn signal_endpoint(base: &str, signal: &str) -> Option<String> {
    let variable = format!("OTEL_EXPORTER_OTLP_{}_ENDPOINT", signal.to_uppercase());
    if env::var_os(variable).is_some() {
        return None;
    }
    return Some(format!("{}/v1/{signal}", base.trim_end_matches('/')));
}

impl Exporters {
    pub fn start(service: Service, endpoint: &str, log_filter: &str) -> Result<(Self, Vec<BoxedLayer>), Error> {
        let resource = Resource::builder()
            .with_service_name(service.name)
            .with_attribute(KeyValue::new(SERVICE_VERSION_ATTRIBUTE, service.version))
            .build();
        let mut directives = log_filter.to_owned();
        for target in TARGETS_OFF {
            directives.push(',');
            directives.push_str(target);
            directives.push_str("=off");
        }
        let filter = || EnvFilter::try_new(&directives).map_err(Error::LogFilter);

        let mut spans = SpanExporter::builder().with_http();
        if let Some(endpoint) = signal_endpoint(endpoint, "traces") {
            spans = spans.with_endpoint(endpoint);
        }
        let tracer_provider = SdkTracerProvider::builder()
            .with_resource(resource.clone())
            .with_batch_exporter(spans.build().map_err(Error::Exporter)?)
            .build();
        global::set_text_map_propagator(TraceContextPropagator::new());
        global::set_tracer_provider(tracer_provider.clone());

        let mut logs = LogExporter::builder().with_http();
        if let Some(endpoint) = signal_endpoint(endpoint, "logs") {
            logs = logs.with_endpoint(endpoint);
        }
        let logger_provider = SdkLoggerProvider::builder()
            .with_resource(resource.clone())
            .with_batch_exporter(logs.build().map_err(Error::Exporter)?)
            .build();

        let mut metrics = MetricExporter::builder().with_http();
        if let Some(endpoint) = signal_endpoint(endpoint, "metrics") {
            metrics = metrics.with_endpoint(endpoint);
        }
        let meter_provider = SdkMeterProvider::builder()
            .with_resource(resource)
            .with_periodic_exporter(metrics.build().map_err(Error::Exporter)?)
            .build();
        global::set_meter_provider(meter_provider.clone());

        let layers = vec![
            tracing_opentelemetry::layer()
                .with_tracer(tracer_provider.tracer(service.name))
                .with_filter(filter()?)
                .boxed(),
            OpenTelemetryTracingBridge::new(&logger_provider)
                .with_filter(filter()?)
                .boxed(),
        ];

        let exporters = Self {
            tracer_provider,
            meter_provider,
            logger_provider,
            #[cfg(target_os = "linux")]
            _process_metrics: crate::process::Instruments::register(&crate::meter()),
        };
        return Ok((exporters, layers));
    }
}

impl Drop for Exporters {
    fn drop(&mut self) {
        if let Err(error) = self.tracer_provider.shutdown() {
            eprintln!("telemetry: shutting down traces: {error}");
        }
        if let Err(error) = self.meter_provider.shutdown() {
            eprintln!("telemetry: shutting down metrics: {error}");
        }
        if let Err(error) = self.logger_provider.shutdown() {
            eprintln!("telemetry: shutting down logs: {error}");
        }
    }
}
