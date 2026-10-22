use std::process::ExitCode;

use scanpay_config::{ConfigError, DbConfig, ProcessEnv, load_dotenv};

fn main() -> ExitCode {
    let mut errors = ConfigError::default();
    errors.absorb(load_dotenv());
    errors.absorb(DbConfig::from_env(&ProcessEnv));
    if !errors.is_empty() {
        eprint!("{errors}");
        return ExitCode::FAILURE;
    }
    println!("Scanpay migrate: configuration valid; no migrations yet (no chunk is frozen)");
    ExitCode::SUCCESS
}
