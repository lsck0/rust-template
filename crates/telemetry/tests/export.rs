use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

use opentelemetry::trace::TraceContextExt;
use telemetry::{FunctionTimer, Service, Settings, init_with};
use tracing_opentelemetry::OpenTelemetrySpanExt;

const SERVICE: Service = Service {
    name: "telemetry-export-test",
    version: "0.0.0",
};
const PYROSCOPE_PUSH_PATH: &str = "/push.v1.PusherService/Push";

struct Request {
    path: String,
    body: Vec<u8>,
}

fn answer(stream: TcpStream, requests: &Mutex<Vec<Request>>) -> Option<()> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).ok()?;
    let path = request_line.split_whitespace().nth(1)?.split('?').next()?.to_owned();

    let mut content_length = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        if header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse().ok()?;
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).ok()?;
    requests.lock().ok()?.push(Request { path, body });

    let mut stream = stream;
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        .ok()?;
    return Some(());
}

fn receiver() -> (String, Arc<Mutex<Vec<Request>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let base = format!("http://{}", listener.local_addr().expect("bound"));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&requests);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let requests = Arc::clone(&recorded);
            thread::spawn(move || answer(stream, &requests));
        }
    });
    return (base, requests);
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    return haystack.windows(needle.len()).any(|window| window == needle);
}

#[test]
fn every_signal_reaches_the_receiver() {
    let (base, requests) = receiver();
    let settings = Settings {
        log_filter: "info".to_owned(),
        log_format: None,
        otlp_endpoint: Some(base.clone()),
        pyroscope_server_address: Some(base),
        pyroscope_basic_auth_user: None,
        pyroscope_basic_auth_password: None,
        pyroscope_tenant_id: None,
    };
    let telemetry = init_with(SERVICE, &settings).expect("first install in this process");
    assert!(telemetry.exports_otlp());
    assert!(telemetry.profiles());

    let span = tracing::info_span!("work");
    let trace_id = span.context().span().span_context().trace_id().to_bytes();
    span.in_scope(|| {
        tracing::info!("inside the span");
        drop(FunctionTimer::start("export::work"));
    });
    drop(span);
    drop(telemetry);

    let requests = std::mem::take(&mut *requests.lock().expect("receiver alive"));
    let body_of = |path: &str| -> Vec<u8> {
        return requests
            .iter()
            .filter(|request| request.path == path)
            .flat_map(|request| request.body.clone())
            .collect();
    };

    let traces = body_of("/v1/traces");
    let logs = body_of("/v1/logs");
    let metrics = body_of("/v1/metrics");
    assert!(contains(&traces, b"work"), "the span is exported");
    assert!(contains(&traces, &trace_id), "under its trace id");
    assert!(contains(&logs, b"inside the span"), "the event is exported");
    assert!(contains(&logs, &trace_id), "the log record carries the span's trace id");
    assert!(
        contains(&metrics, b"function.duration"),
        "the timer's histogram is exported"
    );
    if cfg!(target_os = "linux") {
        assert!(
            contains(&metrics, b"process.memory.usage"),
            "process metrics are exported"
        );
    }
    assert!(
        requests.iter().any(|request| request.path == PYROSCOPE_PUSH_PATH),
        "the profiler pushed its last profile",
    );
    for target in ["Pyroscope", "reqwest", "hyper"] {
        assert!(
            !contains(&logs, target.as_bytes()),
            "{target} stays out of the exported logs"
        );
    }
}
