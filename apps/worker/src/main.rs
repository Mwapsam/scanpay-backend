use std::process::ExitCode;

use scanpay_config::DbConfig;
use scanpay_telemetry::Service;
use tracing::{error, info};

const SERVICE: Service = Service {
    name: "scanpay-worker",
    version: env!("CARGO_PKG_VERSION"),
};

fn main() -> ExitCode {
    let (db, telemetry) = match scanpay_telemetry::start(SERVICE, DbConfig::from_env) {
        Ok(started) => started,
        Err(code) => return code,
    };
    scanpay_telemetry::run(
        telemetry,
        scanpay_db::with_pool(db, async |pool| {
            if let Err(err) = scanpay_db::check_schema(pool).await {
                error!(error = %err, "database startup failed");
                return ExitCode::FAILURE;
            }
            info!("no jobs implemented yet; exiting");
            ExitCode::SUCCESS
        }),
    )
}
