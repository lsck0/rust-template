# Rust Template

cargo-generate template for a Rust workspace (library and binary at the root, crates under `crates/`) with tests,
benchmarks, fuzzing, kani proofs, flux refinement types and Grafana-stack telemetry wired into one cargo-make entry
point and CI.

- `crates/macros`: procedural macros; `#[derive(FromEnv)]` reads a struct from environment variables, with a typed
  error, and `#[timed]` records a function's wall time as a metric.
- `crates/telemetry`: one `telemetry::init` call in `main` sets up `tracing` to stdout, and, when the standard variables
  are set, OTLP logs, traces and metrics (to Loki, Tempo and Mimir through an OpenTelemetry collector such as Alloy) and
  Pyroscope profiles. Log records carry their span's trace id, and Linux processes report CPU, memory and thread
  metrics. The crate documentation lists every variable; the cargo features `otlp`, `profiling` and `tls` (all default)
  drop what a project does not use.

Generate a project:

```bash
cargo generate --git https://github.com/lsck0/rust-template
```

Enter the environment (the pinned toolchain, cargo-make, cargo-deny, cargo-tarpaulin, mold, nushell, typos, perf,
hotspot and the git hooks) with `direnv allow`, or `devenv shell` without direnv. kani, flux and cargo-afl are not in
nixpkgs; install them separately for `cargo make kani`, `flux` and `fuzz`.

List the tasks, then run the gates CI runs:

```bash
cargo make
cargo make check
```

Run the CI workflow itself in containers with [act](https://github.com/nektos/act), all jobs or the named ones; the
runner image is `.github/act/runner.Dockerfile`:

```bash
cargo make ci-local
cargo make ci-local check kani
```

See the binary's logs, traces, metrics and profiles in a local Grafana at <http://localhost:3000>:

```bash
cargo make lgtm          # in one terminal: the grafana/otel-lgtm container
cargo make run-observed  # in another
```

To send to a real stack instead, set `OTEL_EXPORTER_OTLP_ENDPOINT` (plus `OTEL_EXPORTER_OTLP_HEADERS` for auth) and
`PYROSCOPE_SERVER_ADDRESS`.
