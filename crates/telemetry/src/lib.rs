//! Logging and tracing setup, and the startup sequence every binary uses.
//!
//! Libraries only emit `tracing` events and spans; this crate is the one
//! place that decides where they go. Rules for everything that logs:
//!
//! - Spans use `#[instrument(skip_all)]` or `info_span!` with explicit
//!   fields; arguments are never recorded wholesale.
//! - No secrets, tokens, OTPs, PINs or personal data (phone numbers, names)
//!   in fields or messages (05 §4). Span fields also leave the process when
//!   OpenTelemetry export is on, so the same rule covers them.
//! - Audit records go to the `shared` audit table, never to logs. Logs are
//!   for operating the system; they are not evidence.
//!
//! OpenTelemetry (05 §1): spans are exported over OTLP/HTTP when
//! `OTEL_EXPORTER_OTLP_ENDPOINT` is set, and not at all otherwise. Which
//! collector receives them is a deployment decision, not a code change.
//! Service identity comes from the exported resource (`service.name`,
//! `service.version`), so request spans are trace roots, not children of
//! one process-long span.

use std::{future::Future, process::ExitCode, time::Duration};

use opentelemetry::{KeyValue, trace::TracerProvider as _};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use scanpay_config::{ConfigError, Env, ProcessEnv, load_dotenv};
use tracing::{Instrument, Span, error, info, info_span, warn};
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

/// Used when `RUST_LOG` is unset or invalid.
///
/// - `sqlx=warn`: sqlx logs every statement at `info`; at `warn` only slow
///   statements appear. Bind values are never logged.
/// - `hyper=warn,tower=warn`: connection-level noise.
/// - `tower_http=info` is needed, not redundant: directive targets match by
///   plain string prefix (tracing-subscriber `env/directive.rs`), so
///   `tower=warn` alone would also silence `tower_http`'s request logs.
///
/// Per-crate directives (`scanpay_payments=debug`) are how one area's detail
/// is raised; there are no separate logger objects.
pub const DEFAULT_FILTER: &str = "info,sqlx=warn,hyper=warn,tower=warn,tower_http=info";

/// The most [`run`] waits for blocking tasks when the runtime stops.
pub const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Which binary is running. `version` is the binary's own
/// `env!("CARGO_PKG_VERSION")`. Build commit and time join it once CI exists
/// to supply them.
#[derive(Debug, Clone, Copy)]
pub struct Service {
    pub name: &'static str,
    pub version: &'static str,
}

/// The standard OpenTelemetry base endpoint. It is validated here and the
/// exporter is given the result (`<endpoint>/v1/traces`) explicitly, so it
/// never reads a different, unvalidated value from the environment.
pub const OTLP_ENDPOINT_VAR: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";

/// The per-signal override. Refused when present: the endpoint passed in
/// code would silently win over it, and one setting is enough.
pub const OTLP_TRACES_ENDPOINT_VAR: &str = "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable lines, for local development.
    Text,
    /// One JSON object per line, for log shipping.
    Json,
}

/// What [`init`] will install, after validation.
#[derive(Debug)]
pub struct LogSettings {
    pub filter: EnvFilter,
    pub format: LogFormat,
    /// The OTLP/HTTP traces URL, when spans are exported.
    pub export_endpoint: Option<String>,
}

/// Reads `RUST_LOG`, `LOG_FORMAT` (`text` or `json`, default `text`) and
/// `OTEL_EXPORTER_OTLP_ENDPOINT` (optional, `http://` or `https://`; leading
/// and trailing whitespace, such as a CR from a Windows-edited `.env`, is
/// removed).
/// Always returns usable settings: an invalid value falls back to its
/// default (export off) and is reported in the returned problems.
pub fn log_settings(env: &impl Env) -> (LogSettings, ConfigError) {
    let mut problems = ConfigError::default();

    let filter = match problems.absorb(env.var("RUST_LOG")).flatten() {
        None => EnvFilter::new(DEFAULT_FILTER),
        Some(directives) => EnvFilter::try_new(&directives).unwrap_or_else(|_| {
            problems.push("RUST_LOG is not a valid filter (for example info,sqlx=warn)");
            EnvFilter::new(DEFAULT_FILTER)
        }),
    };

    let format = match problems
        .absorb(env.var("LOG_FORMAT"))
        .flatten()
        .as_deref()
        .map(str::trim)
    {
        None | Some("text") => LogFormat::Text,
        Some("json") => LogFormat::Json,
        Some(_) => {
            problems.push("LOG_FORMAT must be text or json");
            LogFormat::Text
        }
    };

    if env.is_set(OTLP_TRACES_ENDPOINT_VAR) {
        problems.push(format!(
            "{OTLP_TRACES_ENDPOINT_VAR} is set; unset it and use {OTLP_ENDPOINT_VAR}"
        ));
    }
    let export_endpoint = problems
        .absorb(env.var(OTLP_ENDPOINT_VAR))
        .flatten()
        .and_then(|raw| {
            let endpoint = raw.trim();
            let valid = url::Url::parse(endpoint)
                .is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.has_host());
            if !valid {
                problems.push(format!(
                    "{OTLP_ENDPOINT_VAR} must be an http:// or https:// URL"
                ));
                return None;
            }
            Some(format!("{}/v1/traces", endpoint.trim_end_matches('/')))
        });

    (
        LogSettings {
            filter,
            format,
            export_endpoint,
        },
        problems,
    )
}

/// Running telemetry for one process. Dropping it flushes and stops the span
/// exporter, so keep it alive until `main` returns, and drop it outside any
/// async runtime (the flush blocks).
#[must_use = "dropping Telemetry stops span export"]
pub struct Telemetry {
    /// The `service` span around startup, the binary's own work and
    /// shutdown. Request spans are not inside it (see the crate docs).
    root: Span,
    provider: Option<SdkTracerProvider>,
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        // End the root span first, so it is exported before the exporter stops.
        drop(std::mem::replace(&mut self.root, Span::none()));
        if let Some(provider) = self.provider.take()
            && let Err(err) = provider.shutdown()
        {
            warn!(error = %err, "span export did not flush cleanly");
        }
    }
}

/// Installs the global subscriber: the console layer (text or JSON) and, when
/// configured, the OTLP span exporter, all behind the `RUST_LOG` filter.
/// Always installs a console layer, so problems in these settings can still be
/// logged; those problems are returned. A second call (for example in tests)
/// leaves the first subscriber in place.
pub fn init(env: &impl Env, service: Service) -> (Option<SdkTracerProvider>, ConfigError) {
    let (settings, mut problems) = log_settings(env);

    let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> = vec![match settings.format {
        LogFormat::Text => fmt::layer().boxed(),
        LogFormat::Json => fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .boxed(),
    }];

    let provider = settings.export_endpoint.and_then(|endpoint| {
        match SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()
        {
            Ok(exporter) => Some(
                SdkTracerProvider::builder()
                    .with_batch_exporter(exporter)
                    .with_resource(
                        Resource::builder()
                            .with_service_name(service.name)
                            .with_attribute(KeyValue::new("service.version", service.version))
                            .build(),
                    )
                    .build(),
            ),
            Err(_) => {
                problems.push(format!(
                    "the OTLP exporter for {OTLP_ENDPOINT_VAR} could not be built"
                ));
                None
            }
        }
    });
    if let Some(provider) = &provider {
        layers.push(
            tracing_opentelemetry::layer()
                .with_tracer(provider.tracer(service.name))
                .boxed(),
        );
    }

    // try_init fails only when a subscriber is already installed.
    let _ = tracing_subscriber::registry()
        .with(layers.with_filter(settings.filter))
        .try_init();
    (provider, problems)
}

/// The startup sequence. One subscriber is installed, once; no bootstrap
/// logger and no reinstall are needed, because nothing is logged before it:
///
/// 1. load `./.env`, so logging variables in it take effect. Its problems
///    are collected, not lost, and a malformed file is fatal (step 4);
/// 2. install logging and span export, then log `starting` with the
///    service name and version;
/// 3. build the configuration;
/// 4. on any problem from 1–3, log each one and return exit code 1.
///
/// Call it before starting an async runtime: the OTLP exporter's blocking
/// HTTP client must not be created or dropped inside one. [`run`] then
/// owns the runtime.
pub fn start<T>(
    service: Service,
    build: impl FnOnce(&ProcessEnv) -> Result<T, ConfigError>,
) -> Result<(T, Telemetry), ExitCode> {
    let mut problems = ConfigError::default();
    problems.absorb(load_dotenv());
    let (provider, logging_problems) = init(&ProcessEnv, service);
    problems.extend(logging_problems);

    let telemetry = Telemetry {
        root: info_span!("service", service = service.name, version = service.version),
        provider,
    };
    telemetry.root.in_scope(|| info!("starting"));
    let config = telemetry
        .root
        .in_scope(|| problems.absorb(build(&ProcessEnv)));
    match config {
        Some(config) if problems.is_empty() => Ok((config, telemetry)),
        _ => {
            telemetry.root.in_scope(|| {
                for problem in problems.problems() {
                    error!(%problem, "invalid configuration");
                }
            });
            // `telemetry` drops here, flushing anything exported.
            Err(ExitCode::FAILURE)
        }
    }
}

/// Runs the binary's async work in the service span, in the order that
/// shutdown needs: build the runtime, run `main` to completion, shut the
/// runtime down, and only then drop `telemetry`, so its final flush (which
/// blocks) happens outside the runtime and includes everything.
///
/// Runtime shutdown cancels remaining tasks and waits at most
/// [`RUNTIME_SHUTDOWN_TIMEOUT`] for blocking ones, so a stuck task cannot
/// keep the process alive past the orchestrator's kill deadline.
pub fn run(telemetry: Telemetry, main: impl Future<Output = ExitCode>) -> ExitCode {
    let code = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => {
            let code = runtime.block_on(main.instrument(telemetry.root.clone()));
            runtime.shutdown_timeout(RUNTIME_SHUTDOWN_TIMEOUT);
            code
        }
        Err(err) => {
            error!(parent: &telemetry.root, error = %err, "failed to start the async runtime");
            ExitCode::FAILURE
        }
    };
    drop(telemetry);
    code
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn settings(pairs: &[(&'static str, &'static str)]) -> (LogSettings, ConfigError) {
        log_settings(&pairs.iter().copied().collect::<HashMap<_, _>>())
    }

    #[test]
    fn defaults() {
        let (settings, problems) = settings(&[]);
        assert!(problems.is_empty());
        assert_eq!(settings.format, LogFormat::Text);
        assert_eq!(settings.export_endpoint, None);
        assert_eq!(
            settings.filter.to_string(),
            EnvFilter::new(DEFAULT_FILTER).to_string()
        );
    }

    #[test]
    fn json_custom_filter_and_export() {
        let (settings, problems) = settings(&[
            ("LOG_FORMAT", "json"),
            ("RUST_LOG", "debug,sqlx=info"),
            (OTLP_ENDPOINT_VAR, "http://127.0.0.1:4318"),
        ]);
        assert!(problems.is_empty());
        assert_eq!(settings.format, LogFormat::Json);
        assert_eq!(
            settings.export_endpoint.as_deref(),
            Some("http://127.0.0.1:4318/v1/traces")
        );
        assert!(settings.filter.to_string().contains("sqlx=info"));
    }

    /// The exporter is given the trimmed value, so stray whitespace (a CR
    /// from a Windows-edited `.env`) cannot pass validation and then fail
    /// every export at runtime.
    #[test]
    fn export_endpoint_is_trimmed_and_given_the_traces_path() {
        for raw in [" http://collector:4318\r", "http://collector:4318/"] {
            let (settings, problems) = settings(&[(OTLP_ENDPOINT_VAR, raw)]);
            assert!(problems.is_empty(), "{raw:?}");
            assert_eq!(
                settings.export_endpoint.as_deref(),
                Some("http://collector:4318/v1/traces"),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn per_signal_endpoint_is_refused() {
        let (_, problems) =
            settings(&[(OTLP_TRACES_ENDPOINT_VAR, "http://collector:4318/v1/traces")]);
        assert_eq!(
            problems.problems(),
            [
                "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT is set; unset it and use OTEL_EXPORTER_OTLP_ENDPOINT"
            ]
        );
    }

    /// The default filter keeps request logs while capping connection noise:
    /// checked by sending events through a real subscriber with that filter.
    #[test]
    fn default_filter_keeps_request_logs_and_caps_noise() {
        use std::sync::{Arc, Mutex};

        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for Capture {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let capture = Capture::default();
        let writer = capture.clone();
        let subscriber = tracing_subscriber::registry().with(
            fmt::layer()
                .with_writer(move || writer.clone())
                .with_filter(EnvFilter::new(DEFAULT_FILTER)),
        );
        tracing::subscriber::with_default(subscriber, || {
            info!(target: "tower_http::trace::on_response", "kept-request");
            info!(target: "scanpay_payments", "kept-domain");
            info!(target: "tower::buffer", "dropped-tower");
            info!(target: "hyper::proto::h1", "dropped-hyper");
            info!(target: "sqlx::query", "dropped-sqlx");
            warn!(target: "sqlx::query", "kept-slow-query");
        });

        let out = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
        for kept in ["kept-request", "kept-domain", "kept-slow-query"] {
            assert!(out.contains(kept), "{kept} missing from:\n{out}");
        }
        for dropped in ["dropped-tower", "dropped-hyper", "dropped-sqlx"] {
            assert!(
                !out.contains(dropped),
                "{dropped} should be filtered:\n{out}"
            );
        }
    }

    #[test]
    fn bad_values_fall_back_and_are_reported() {
        let (settings, problems) = settings(&[
            ("LOG_FORMAT", "xml"),
            ("RUST_LOG", "info,[unclosed"),
            (OTLP_ENDPOINT_VAR, "collector:4318"),
        ]);
        assert_eq!(settings.format, LogFormat::Text);
        assert_eq!(settings.export_endpoint, None);
        assert_eq!(
            settings.filter.to_string(),
            EnvFilter::new(DEFAULT_FILTER).to_string()
        );
        assert_eq!(
            problems.problems(),
            [
                "RUST_LOG is not a valid filter (for example info,sqlx=warn)",
                "LOG_FORMAT must be text or json",
                "OTEL_EXPORTER_OTLP_ENDPOINT must be an http:// or https:// URL",
            ]
        );
    }
}
