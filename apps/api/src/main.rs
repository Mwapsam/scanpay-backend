mod config;

use std::{
    future::{Future, IntoFuture},
    process::ExitCode,
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    http::{HeaderName, Request},
    routing::get,
};
use scanpay_telemetry::Service;
use tokio::{net::TcpListener, sync::oneshot};
use tower::{ServiceBuilder, util::MapRequestLayer};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::{Level, error, info, info_span, warn};

use crate::config::ApiConfig;

const SERVICE: Service = Service {
    name: "scanpay-api",
    version: env!("CARGO_PKG_VERSION"),
};

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
/// Longest caller-supplied request id that is kept.
const MAX_REQUEST_ID_LEN: usize = 128;

/// How long in-flight requests get after the shutdown signal before they are
/// dropped. With the pool close (3 s), runtime stop (2 s) and span flush
/// (5 s) bounds, shutdown fits inside the usual 30 s an orchestrator waits
/// before SIGKILL.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(15);

/// The HTTP router. Every request gets an `x-request-id`: the caller's own if
/// it is safe to log (see [`is_loggable_request_id`]), otherwise a new UUID.
/// It is recorded on an `http.request` span with method and path (never
/// headers or bodies, which can carry tokens and personal data), and echoed
/// in the response. Each request span is a trace root, so every request is
/// its own trace.
fn app() -> Router {
    let trace = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            let request_id = request
                .headers()
                .get(&X_REQUEST_ID)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            info_span!(
                parent: None,
                "http.request",
                method = %request.method(),
                path = request.uri().path(),
                request_id,
            )
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    Router::new().route("/health", get(health)).layer(
        ServiceBuilder::new()
            .layer(MapRequestLayer::new(drop_unloggable_request_id))
            .layer(SetRequestIdLayer::new(X_REQUEST_ID, MakeRequestUuid))
            .layer(trace)
            .layer(PropagateRequestIdLayer::new(X_REQUEST_ID)),
    )
}

/// A caller's id goes into every log line and span of its request, so only
/// short ids of `[A-Za-z0-9._-]` are kept: no newlines or control characters
/// that could forge log lines, and nothing large.
fn is_loggable_request_id(id: &[u8]) -> bool {
    (1..=MAX_REQUEST_ID_LEN).contains(&id.len())
        && id
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Removes every caller `x-request-id` unless there is exactly one and it is
/// loggable; the next layer then assigns a fresh UUID.
fn drop_unloggable_request_id(mut request: Request<Body>) -> Request<Body> {
    let mut ids = request.headers().get_all(&X_REQUEST_ID).iter();
    let keep = matches!(
        (ids.next(), ids.next()),
        (Some(id), None) if is_loggable_request_id(id.as_bytes())
    );
    if !keep {
        request.headers_mut().remove(&X_REQUEST_ID);
    }
    request
}

/// Liveness only: the process is up and serving. Readiness checks (database,
/// providers) come with the endpoints that need them.
async fn health() -> &'static str {
    "ok"
}

/// Telemetry and configuration first, outside any async runtime; then
/// `scanpay_telemetry::run` owns the runtime and the shutdown order.
fn main() -> ExitCode {
    let (config, telemetry) = match scanpay_telemetry::start(SERVICE, ApiConfig::from_env) {
        Ok(started) => started,
        Err(code) => return code,
    };
    scanpay_telemetry::run(telemetry, run(config))
}

/// Database first (connect, role, schema), so the server only accepts
/// traffic once it can serve it; then bind and serve until shutdown.
async fn run(config: ApiConfig) -> ExitCode {
    let bind_addr = config.bind_addr;
    scanpay_db::with_pool(config.db, async |pool| {
        if let Err(err) = scanpay_db::check_schema(pool).await {
            error!(error = %err, "database startup failed");
            return ExitCode::FAILURE;
        }
        let listener = match TcpListener::bind(bind_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                error!(%bind_addr, error = %err, "failed to bind");
                return ExitCode::FAILURE;
            }
        };
        info!(%bind_addr, "listening");

        match serve(listener, app(), shutdown_signal(), SHUTDOWN_GRACE).await {
            Ok(()) => {
                info!("stopped");
                ExitCode::SUCCESS
            }
            Err(err) => {
                error!(error = %err, "server failed");
                ExitCode::FAILURE
            }
        }
    })
    .await
}

/// Serves until `shutdown` resolves, then stops accepting connections and
/// lets in-flight requests finish (05 §6), for at most `grace`. After that
/// the remaining requests are dropped, so a hung request or a client that
/// never finishes sending cannot hold shutdown open until SIGKILL.
async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
    grace: Duration,
) -> std::io::Result<()> {
    let (signalled, on_signal) = oneshot::channel::<()>();
    let server = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            shutdown.await;
            let _ = signalled.send(());
        })
        .into_future();
    let deadline = async move {
        // An error means the server already finished, so the select below
        // has taken that branch; never fire in that case.
        if on_signal.await.is_err() {
            std::future::pending::<()>().await;
        }
        tokio::time::sleep(grace).await;
    };
    tokio::select! {
        served = server => served,
        () = deadline => {
            warn!(grace_secs = grace.as_secs(), "in-flight requests did not finish in time; dropping them");
            Ok(())
        }
    }
}

/// Resolves on Ctrl-C, or SIGTERM on Unix.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            error!(error = %err, "cannot listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                error!(error = %err, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    info!("shutdown signal received; finishing in-flight requests");
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{app, is_loggable_request_id, serve};
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use http_body_util::BodyExt;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::{Notify, oneshot},
    };
    use tower::ServiceExt;

    /// A server with a `/held` route that reports when a request reaches it
    /// and answers only when released, plus a shutdown trigger that reports
    /// when the server has seen it. No sleeps: each step waits for the last.
    struct Held {
        addr: std::net::SocketAddr,
        started: Arc<Notify>,
        release: Arc<Notify>,
        stop: oneshot::Sender<()>,
        seen: oneshot::Receiver<()>,
        server: tokio::task::JoinHandle<std::io::Result<()>>,
    }

    async fn held_server(grace: Duration) -> Held {
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let route = {
            let (started, release) = (started.clone(), release.clone());
            get(move || async move {
                started.notify_one();
                release.notified().await;
                "done"
            })
        };
        let router = Router::new().route("/held", route).merge(app());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (stop, stopped) = oneshot::channel::<()>();
        let (seen_tx, seen) = oneshot::channel::<()>();
        let shutdown = async move {
            let _ = stopped.await;
            let _ = seen_tx.send(());
        };
        let server = tokio::spawn(serve(listener, router, shutdown, grace));
        Held {
            addr,
            started,
            release,
            stop,
            seen,
            server,
        }
    }

    async fn send_held_request(addr: std::net::SocketAddr) -> TcpStream {
        let mut client = TcpStream::connect(addr).await.unwrap();
        client
            .write_all(b"GET /held HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        client
    }

    #[tokio::test]
    async fn shutdown_lets_in_flight_requests_finish() {
        let held = held_server(Duration::from_secs(30)).await;
        let mut client = send_held_request(held.addr).await;

        // The handler is running; signal shutdown and wait until the server
        // has seen it, and only then let the handler answer.
        held.started.notified().await;
        held.stop.send(()).unwrap();
        held.seen.await.unwrap();
        held.release.notify_one();

        let mut response = String::new();
        client.read_to_string(&mut response).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.ends_with("done"), "{response}");

        let finished = tokio::time::timeout(Duration::from_secs(5), held.server).await;
        assert!(
            matches!(finished, Ok(Ok(Ok(())))),
            "server did not stop cleanly"
        );
        assert!(
            TcpStream::connect(held.addr).await.is_err(),
            "still accepting after shutdown"
        );
    }

    #[tokio::test]
    async fn shutdown_does_not_wait_past_the_grace_period() {
        let held = held_server(Duration::from_millis(200)).await;
        let _client = send_held_request(held.addr).await;

        held.started.notified().await;
        held.stop.send(()).unwrap();
        // The handler is never released; the deadline must end the server.
        let finished = tokio::time::timeout(Duration::from_secs(5), held.server).await;
        assert!(
            matches!(finished, Ok(Ok(Ok(())))),
            "server did not stop at the deadline"
        );
    }

    fn request(uri: &str, request_ids: &[&str]) -> Request<Body> {
        let mut builder = Request::builder().uri(uri);
        for id in request_ids {
            builder = builder.header("x-request-id", *id);
        }
        builder.body(Body::empty()).expect("valid request")
    }

    async fn response_request_id(request_ids: &[&str]) -> String {
        let response = app()
            .oneshot(request("/health", request_ids))
            .await
            .unwrap();
        response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned()
    }

    fn is_uuid(id: &str) -> bool {
        id.len() == 36 && id.matches('-').count() == 4
    }

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let response = app()
            .oneshot(request("/health", &[]))
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body should be readable")
            .to_bytes();

        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn every_response_carries_a_uuid_request_id() {
        assert!(is_uuid(&response_request_id(&[]).await));
    }

    #[tokio::test]
    async fn a_loggable_caller_request_id_is_kept() {
        assert_eq!(
            response_request_id(&["caller-id_1.2"]).await,
            "caller-id_1.2"
        );
    }

    #[tokio::test]
    async fn unloggable_caller_request_ids_are_replaced() {
        let long = "a".repeat(129);
        for ids in [
            vec!["has space"],
            vec!["line\tbreak"],
            vec!["semi;colon"],
            vec![long.as_str()],
            vec!["first", "second"],
        ] {
            let id = response_request_id(&ids).await;
            assert!(is_uuid(&id), "{ids:?} gave {id}");
        }
    }

    #[test]
    fn loggable_request_id_rule() {
        assert!(is_loggable_request_id(b"abc-123_X.y"));
        assert!(is_loggable_request_id("a".repeat(128).as_bytes()));
        assert!(!is_loggable_request_id(b""));
        assert!(!is_loggable_request_id("a".repeat(129).as_bytes()));
        assert!(!is_loggable_request_id(b"a\nb"));
        assert!(!is_loggable_request_id("é".as_bytes()));
    }
}
