use std::process::ExitCode;

use scanpay_config::DbConfig;

fn main() -> ExitCode {
    if let Err(code) = scanpay_config::load(DbConfig::from_env) {
        return code;
    }
    println!("Scanpay migrate: configuration valid; no migrations yet (no chunk is frozen)");
    ExitCode::SUCCESS
}
