use std::process::ExitCode;

use scanpay_config::DbConfig;

fn main() -> ExitCode {
    if let Err(code) = scanpay_config::load(DbConfig::from_env) {
        return code;
    }
    println!("Scanpay worker: configuration valid; no jobs implemented yet");
    ExitCode::SUCCESS
}
