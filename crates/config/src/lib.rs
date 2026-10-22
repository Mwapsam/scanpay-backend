//! Startup configuration, validated before anything connects or binds.
//!
//! Loading collects every problem and reports them together, so a bad deploy
//! is fixed in one pass. Secrets (the database password, `.env` contents)
//! never appear in `Debug` output or error messages.

use std::{borrow::Borrow, collections::HashMap, env::VarError, fmt, hash::Hash, str::FromStr};

use sqlx::postgres::PgConnectOptions;

/// Where configuration values come from. Tests use a `HashMap`.
pub trait Env {
    /// `Ok(None)` when `key` is unset or empty; `Err` when it is set but
    /// unusable (for example not valid UTF-8), so it is never mistaken for unset.
    fn var(&self, key: &str) -> Result<Option<String>, ConfigError>;
}

/// The process environment (after [`load_dotenv`], if used).
pub struct ProcessEnv;

impl Env for ProcessEnv {
    fn var(&self, key: &str) -> Result<Option<String>, ConfigError> {
        match std::env::var(key) {
            Ok(v) => Ok(non_empty(v)),
            Err(VarError::NotPresent) => Ok(None),
            Err(VarError::NotUnicode(_)) => {
                Err(ConfigError::new(format!("{key} is set but is not valid UTF-8")))
            }
        }
    }
}

impl<K, V> Env for HashMap<K, V>
where
    K: Borrow<str> + Hash + Eq,
    V: AsRef<str>,
{
    fn var(&self, key: &str) -> Result<Option<String>, ConfigError> {
        Ok(self.get(key).and_then(|v| non_empty(v.as_ref().to_owned())))
    }
}

fn non_empty(v: String) -> Option<String> {
    (!v.trim().is_empty()).then_some(v)
}

/// Loads `.env` from the working directory if there is one. A missing file is
/// fine; a malformed one is a configuration problem, because dotenvy stops at
/// the bad line and every variable after it would silently go unset. The
/// message never includes the file's contents.
pub fn load_dotenv() -> Result<(), ConfigError> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(e) if e.not_found() => Ok(()),
        Err(dotenvy::Error::LineParse(_, _)) => Err(ConfigError::new(
            ".env has a syntax error; variables after the bad line were not loaded",
        )),
        Err(dotenvy::Error::Io(e)) => Err(ConfigError::new(format!(".env could not be read: {e}"))),
        Err(_) => Err(ConfigError::new(".env could not be loaded")),
    }
}

/// Every problem found while loading configuration.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ConfigError {
    problems: Vec<String>,
}

impl ConfigError {
    pub fn new(problem: impl Into<String>) -> Self {
        Self {
            problems: vec![problem.into()],
        }
    }

    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    pub fn push(&mut self, problem: impl Into<String>) {
        self.problems.push(problem.into());
    }

    /// Adds the problems from `result`, returning its value if it had none.
    pub fn absorb<T>(&mut self, result: Result<T, ConfigError>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(other) => {
                self.problems.extend(other.problems);
                None
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.problems.is_empty()
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "invalid configuration:")?;
        for problem in &self.problems {
            writeln!(f, "  - {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

/// Parses an optional variable, falling back to `default` when unset.
pub fn optional<T: FromStr>(
    env: &impl Env,
    key: &str,
    default: T,
    expected: &str,
) -> Result<T, ConfigError> {
    match env.var(key)? {
        None => Ok(default),
        Some(raw) => raw
            .trim()
            .parse()
            .map_err(|_| ConfigError::new(format!("{key} must be {expected}"))),
    }
}

const DEFAULT_MAX_CONNECTIONS: u32 = 5;
const MAX_CONNECTIONS_RANGE: std::ops::RangeInclusive<u32> = 1..=100;

/// libpq variables that sqlx's URL parser silently uses as defaults for
/// anything the URL leaves out (sqlx-postgres `PgConnectOptions::new_without_pgpass`).
/// Scanpay takes connection settings only from `DATABASE_URL`, so any of these
/// being set is refused rather than quietly merged in.
const LIBPQ_VARS: [&str; 12] = [
    "PGHOST",
    "PGHOSTADDR",
    "PGPORT",
    "PGUSER",
    "PGPASSWORD",
    "PGDATABASE",
    "PGSSLMODE",
    "PGSSLROOTCERT",
    "PGSSLCERT",
    "PGSSLKEY",
    "PGAPPNAME",
    "PGOPTIONS",
];

/// PostgreSQL connection settings.
#[derive(Clone)]
pub struct DbConfig {
    connect_options: PgConnectOptions,
    max_connections: u32,
}

impl DbConfig {
    /// Reads `DATABASE_URL` (required, `postgres://` or `postgresql://`) and
    /// `DATABASE_MAX_CONNECTIONS` (optional, 1–100, default 5), and refuses
    /// any libpq `PG*` variable (see [`LIBPQ_VARS`]).
    ///
    /// Note for tests: the check covers the `Env` given, but sqlx's parser
    /// reads `PG*` from the real process environment regardless, so a test
    /// run is hermetic only in a shell with none of them set. In production
    /// `Env` is the process environment, so the check closes that gap.
    pub fn from_env(env: &impl Env) -> Result<Self, ConfigError> {
        let mut errors = ConfigError::default();

        for key in LIBPQ_VARS {
            match env.var(key) {
                Ok(None) => {}
                Ok(Some(_)) | Err(_) => errors.push(format!(
                    "{key} is set; unset it, connection settings come only from DATABASE_URL"
                )),
            }
        }

        let connect_options = match env.var("DATABASE_URL") {
            Err(e) => {
                errors.absorb::<()>(Err(e));
                None
            }
            Ok(None) => {
                errors.push("DATABASE_URL is required");
                None
            }
            Ok(Some(url)) => {
                let url = url.trim();
                let scheme_ok = url.starts_with("postgres://") || url.starts_with("postgresql://");
                // The parser's own message is not shown: it could echo the URL.
                match PgConnectOptions::from_str(url) {
                    Ok(options) if scheme_ok => Some(options),
                    _ => {
                        errors.push(
                            "DATABASE_URL must be a postgres:// or postgresql:// connection URL",
                        );
                        None
                    }
                }
            }
        };

        let max_connections = errors
            .absorb(optional(
                env,
                "DATABASE_MAX_CONNECTIONS",
                DEFAULT_MAX_CONNECTIONS,
                "a whole number from 1 to 100",
            ))
            .and_then(|n| {
                if MAX_CONNECTIONS_RANGE.contains(&n) {
                    Some(n)
                } else {
                    errors.push("DATABASE_MAX_CONNECTIONS must be a whole number from 1 to 100");
                    None
                }
            });

        // The guard matters: a refused PG* variable is a problem even when
        // both values parsed.
        match (connect_options, max_connections) {
            (Some(connect_options), Some(max_connections)) if errors.is_empty() => Ok(Self {
                connect_options,
                max_connections,
            }),
            _ => Err(errors),
        }
    }

    pub fn connect_options(&self) -> &PgConnectOptions {
        &self.connect_options
    }

    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }
}

impl fmt::Debug for DbConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DbConfig")
            .field("host", &self.connect_options.get_host())
            .field("port", &self.connect_options.get_port())
            .field("database", &self.connect_options.get_database())
            .field("max_connections", &self.max_connections)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "postgres://scanpay:s3cret-pass@127.0.0.1:5432/scanpay";

    fn env(pairs: &[(&'static str, &'static str)]) -> HashMap<&'static str, &'static str> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn loads_with_defaults() {
        let config = DbConfig::from_env(&env(&[("DATABASE_URL", URL)])).unwrap();
        assert_eq!(config.max_connections(), 5);
        assert_eq!(config.connect_options().get_database(), Some("scanpay"));
    }

    #[test]
    fn missing_url_is_reported() {
        let err = DbConfig::from_env(&env(&[])).unwrap_err();
        assert_eq!(err.problems(), ["DATABASE_URL is required"]);
    }

    #[test]
    fn empty_value_counts_as_unset() {
        let err = DbConfig::from_env(&env(&[("DATABASE_URL", "  ")])).unwrap_err();
        assert_eq!(err.problems(), ["DATABASE_URL is required"]);
    }

    #[test]
    fn rejects_non_postgres_urls() {
        for bad in ["mysql://u:p@host/db", "not a url", "http://host/db"] {
            let err = DbConfig::from_env(&env(&[("DATABASE_URL", bad)])).unwrap_err();
            assert_eq!(err.problems().len(), 1, "{bad}");
        }
    }

    #[test]
    fn refuses_libpq_variables() {
        let err = DbConfig::from_env(&env(&[
            ("DATABASE_URL", URL),
            ("PGPASSWORD", "other"),
            ("PGSSLMODE", "disable"),
        ]))
        .unwrap_err();
        assert_eq!(
            err.problems(),
            [
                "PGPASSWORD is set; unset it, connection settings come only from DATABASE_URL",
                "PGSSLMODE is set; unset it, connection settings come only from DATABASE_URL",
            ]
        );
        assert!(!err.to_string().contains("other"));
    }

    #[test]
    fn unusable_value_is_not_treated_as_unset() {
        struct BadUnicode;
        impl Env for BadUnicode {
            fn var(&self, key: &str) -> Result<Option<String>, ConfigError> {
                match key {
                    "DATABASE_URL" => Ok(Some(URL.to_owned())),
                    "DATABASE_MAX_CONNECTIONS" => {
                        Err(ConfigError::new(format!("{key} is set but is not valid UTF-8")))
                    }
                    _ => Ok(None),
                }
            }
        }
        let err = DbConfig::from_env(&BadUnicode).unwrap_err();
        assert_eq!(
            err.problems(),
            ["DATABASE_MAX_CONNECTIONS is set but is not valid UTF-8"]
        );
    }

    #[test]
    fn reports_every_problem_at_once() {
        let err = DbConfig::from_env(&env(&[
            ("DATABASE_URL", "mysql://x"),
            ("DATABASE_MAX_CONNECTIONS", "0"),
        ]))
        .unwrap_err();
        assert_eq!(err.problems().len(), 2);

        let err = DbConfig::from_env(&env(&[("DATABASE_MAX_CONNECTIONS", "lots")])).unwrap_err();
        assert_eq!(err.problems().len(), 2);
    }

    #[test]
    fn max_connections_range() {
        for (value, ok) in [("1", true), ("100", true), ("0", false), ("101", false), ("-1", false)] {
            let result = DbConfig::from_env(&env(&[
                ("DATABASE_URL", URL),
                ("DATABASE_MAX_CONNECTIONS", value),
            ]));
            assert_eq!(result.is_ok(), ok, "{value}");
        }
    }

    #[test]
    fn password_never_shown() {
        let config = DbConfig::from_env(&env(&[("DATABASE_URL", URL)])).unwrap();
        assert!(!format!("{config:?}").contains("s3cret"));

        let err = DbConfig::from_env(&env(&[(
            "DATABASE_URL",
            "mysql://scanpay:s3cret-pass@host/db",
        )]))
        .unwrap_err();
        assert!(!err.to_string().contains("s3cret"));
    }
}
