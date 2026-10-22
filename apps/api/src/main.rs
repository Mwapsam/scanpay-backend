mod config;

use std::process::ExitCode;

use axum::{Router, routing::get};
use tokio::net::TcpListener;

use crate::config::ApiConfig;

fn app() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> ExitCode {
    let config = match scanpay_config::load(ApiConfig::from_env) {
        Ok(config) => config,
        Err(code) => return code,
    };
    let bind_addr = config.bind_addr;

    let (pool_options, connect_options) = config.db.into_pool_parts();
    let pool = match pool_options.connect_with(connect_options).await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("failed to connect to PostgreSQL: {err}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(err) = sqlx::query("SELECT 1").execute(&pool).await {
        eprintln!("PostgreSQL connectivity check failed: {err}");
        return ExitCode::FAILURE;
    }

    println!("Scanpay connected to PostgreSQL successfully");

    let listener = match TcpListener::bind(bind_addr).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("failed to bind to {bind_addr}: {err}");
            return ExitCode::FAILURE;
        }
    };

    println!("Scanpay API listening on http://{bind_addr}");

    if let Err(err) = axum::serve(listener, app()).await {
        eprintln!("API server failed: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("valid request"),
            )
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
}
