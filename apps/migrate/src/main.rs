use std::process::ExitCode;

use scanpay_config::DbConfig;
use scanpay_telemetry::Service;
use tracing::error;

const SERVICE: Service = Service {
    name: "scanpay-migrate",
    version: env!("CARGO_PKG_VERSION"),
};

/// The only binary that changes the schema: applies every pending migration
/// embedded in `scanpay_db::MIGRATOR`, then exits.
fn main() -> ExitCode {
    let (db, telemetry) = match scanpay_telemetry::start(SERVICE, DbConfig::from_env) {
        Ok(started) => started,
        Err(code) => return code,
    };
    scanpay_telemetry::run(
        telemetry,
        scanpay_db::with_pool(db, async |pool| {
            match scanpay_db::apply_migrations(pool).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    error!(error = %err, "migration failed");
                    ExitCode::FAILURE
                }
            }
        }),
    )
}
