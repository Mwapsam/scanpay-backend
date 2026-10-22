use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use scanpay_config::{ConfigError, DbConfig, Env, optional};

const DEFAULT_BIND_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000);

#[derive(Debug)]
pub struct ApiConfig {
    pub db: DbConfig,
    pub bind_addr: SocketAddr,
}

impl ApiConfig {
    /// Reads the database settings plus `API_BIND_ADDR` (optional, a socket
    /// address such as `127.0.0.1:3000`). Reports every problem together.
    pub fn from_env(env: &impl Env) -> Result<Self, ConfigError> {
        let mut errors = ConfigError::default();
        let db = errors.absorb(DbConfig::from_env(env));
        let bind_addr = errors.absorb(optional(
            env,
            "API_BIND_ADDR",
            DEFAULT_BIND_ADDR,
            "a socket address such as 127.0.0.1:3000",
        ));
        match (db, bind_addr) {
            (Some(db), Some(bind_addr)) => Ok(Self { db, bind_addr }),
            _ => Err(errors),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn defaults_bind_address() {
        let env = HashMap::from([("DATABASE_URL", "postgres://u:p@127.0.0.1/scanpay")]);
        let config = ApiConfig::from_env(&env).unwrap();
        assert_eq!(config.bind_addr.to_string(), "127.0.0.1:3000");
    }

    #[test]
    fn collects_database_and_bind_problems() {
        let env = HashMap::from([("API_BIND_ADDR", "bad")]);
        let err = ApiConfig::from_env(&env).unwrap_err();
        assert_eq!(
            err.problems(),
            [
                "DATABASE_URL is required",
                "API_BIND_ADDR must be a socket address such as 127.0.0.1:3000",
            ]
        );
    }
}
