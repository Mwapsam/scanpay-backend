//! Startup configuration, validated before anything connects or binds.
//!
//! Loading collects every problem and reports them together, so a bad deploy
//! is fixed in one pass. Secrets (the database password, `.env` contents)
//! never appear in `Debug` output or error messages. This crate produces no
//! output itself: binaries report problems through `scanpay_telemetry::start`.

use std::{borrow::Borrow, collections::HashMap, env::VarError, fmt, hash::Hash, str::FromStr};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

/// Where configuration values come from. Tests use a `HashMap`.
pub trait Env {
    /// `Ok(None)` when `key` is unset or empty; `Err` when it is set but
    /// unusable (for example not valid UTF-8), so it is never mistaken for unset.
    fn var(&self, key: &str) -> Result<Option<String>, ConfigError>;

    /// Whether `key` is present at all, even with an empty value.
    fn is_set(&self, key: &str) -> bool;
}

/// The process environment (after [`load_dotenv`], if used).
pub struct ProcessEnv;

impl Env for ProcessEnv {
    fn var(&self, key: &str) -> Result<Option<String>, ConfigError> {
        match std::env::var(key) {
            Ok(v) => Ok(non_empty(v)),
            Err(VarError::NotPresent) => Ok(None),
            Err(VarError::NotUnicode(_)) => Err(ConfigError::new(format!(
                "{key} is set but is not valid UTF-8"
            ))),
        }
    }

    fn is_set(&self, key: &str) -> bool {
        std::env::var_os(key).is_some()
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

    fn is_set(&self, key: &str) -> bool {
        self.contains_key(key)
    }
}

fn non_empty(v: String) -> Option<String> {
    (!v.trim().is_empty()).then_some(v)
}

/// Loads `./.env` (the working directory only; parent directories are never
/// searched, unlike `dotenvy::dotenv`). Variables already set in the process
/// keep their values. A missing file is fine; a malformed one is a
/// configuration problem, because dotenvy stops at the bad line and every
/// variable after it would silently go unset. The message never includes the
/// file's contents. Deploy images must not ship a `.env`.
pub fn load_dotenv() -> Result<(), ConfigError> {
    match dotenvy::from_path(".env") {
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

    /// Adds every problem in `other`.
    pub fn extend(&mut self, other: ConfigError) {
        self.problems.extend(other.problems);
    }

    /// Adds the problems from `result`, returning its value if it had none.
    pub fn absorb<T>(&mut self, result: Result<T, ConfigError>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(other) => {
                self.extend(other);
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
const MAX_CONNECTIONS_EXPECTED: &str = "a whole number from 1 to 100";

/// libpq variables that sqlx's URL parser silently uses as defaults for
/// anything the URL leaves out (sqlx-postgres `PgConnectOptions::new_without_pgpass`
/// and `apply_pgpass`). Scanpay takes connection settings only from
/// `DATABASE_URL`, so any of these being present, even empty, is refused
/// rather than quietly merged in.
const LIBPQ_VARS: [&str; 13] = [
    "PGPASSFILE",
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
pub struct DbConfig {
    connect_options: PgConnectOptions,
    max_connections: u32,
    allow_privileged_role: bool,
}

impl DbConfig {
    /// Reads `DATABASE_URL`, `DATABASE_MAX_CONNECTIONS` (optional, 1–100,
    /// default 5) and `DATABASE_ALLOW_PRIVILEGED_ROLE` (optional, `true` or
    /// `false`, default `false`), and refuses any libpq `PG*` variable (see
    /// [`LIBPQ_VARS`]).
    ///
    /// `DATABASE_URL` must be `postgres://user:password@host[:port]/database`
    /// with every one of user, password, host and database written in it, so
    /// sqlx never fills a gap from the OS user name, the server's default
    /// database or a password file. Query parameters that would override
    /// those parts (`?user=`, `?dbname=`, …) are refused; others such as
    /// `?sslmode=` are allowed.
    ///
    /// Tests and the process environment: sqlx's parser always starts from
    /// `PG*` defaults in the real process environment, then overwrites them
    /// with every part of the URL (sqlx-postgres `parse_from_url`). Because the
    /// URL must spell out host, user, password and database, and the password
    /// being present skips any password file, those values never come from the
    /// shell; tests assert only on them and on this function's own problems,
    /// so their outcome does not depend on the shell. In production `Env` is
    /// the process environment, so the `PG*` refusal covers the rest.
    pub fn from_env(env: &impl Env) -> Result<Self, ConfigError> {
        let mut errors = ConfigError::default();

        for key in LIBPQ_VARS {
            if env.is_set(key) {
                errors.push(format!(
                    "{key} is set; unset it, connection settings come only from DATABASE_URL"
                ));
            }
        }

        let connect_options = match env.var("DATABASE_URL") {
            Err(e) => {
                errors.extend(e);
                None
            }
            Ok(None) => {
                errors.push("DATABASE_URL is required");
                None
            }
            Ok(Some(url)) => errors.absorb(check_database_url(url.trim())),
        };

        let max_connections = errors
            .absorb(optional(
                env,
                "DATABASE_MAX_CONNECTIONS",
                DEFAULT_MAX_CONNECTIONS,
                MAX_CONNECTIONS_EXPECTED,
            ))
            .and_then(|n| {
                if MAX_CONNECTIONS_RANGE.contains(&n) {
                    Some(n)
                } else {
                    errors.push(format!(
                        "DATABASE_MAX_CONNECTIONS must be {MAX_CONNECTIONS_EXPECTED}"
                    ));
                    None
                }
            });

        let allow_privileged_role = errors.absorb(optional(
            env,
            "DATABASE_ALLOW_PRIVILEGED_ROLE",
            false,
            "true or false",
        ));

        // The guard matters: a refused PG* variable is a problem even when
        // every value parsed.
        match (connect_options, max_connections, allow_privileged_role) {
            (Some(connect_options), Some(max_connections), Some(allow_privileged_role))
                if errors.is_empty() =>
            {
                Ok(Self {
                    connect_options,
                    max_connections,
                    allow_privileged_role,
                })
            }
            _ => Err(errors),
        }
    }

    /// Whether connecting as a superuser or BYPASSRLS role is allowed. Such a
    /// role is not subject to row-level security, so the default is no; only
    /// local development against the compose superuser opts in.
    pub fn allow_privileged_role(&self) -> bool {
        self.allow_privileged_role
    }

    /// Pool settings and connection options, ready for
    /// `pool_options.connect_with(connect_options)`. Pool setup lives here
    /// so every binary builds its pool the same way.
    pub fn into_pool_parts(self) -> (PgPoolOptions, PgConnectOptions) {
        (
            PgPoolOptions::new().max_connections(self.max_connections),
            self.connect_options,
        )
    }
}

/// Requires every connection setting to be written in the URL itself and
/// reports every missing part at once. Messages never include the URL or
/// any part of it.
fn check_database_url(raw: &str) -> Result<PgConnectOptions, ConfigError> {
    const SHAPE: &str = "DATABASE_URL must be postgres://user:password@host[:port]/database";
    let url = match url::Url::parse(raw) {
        Ok(url) if matches!(url.scheme(), "postgres" | "postgresql") => url,
        _ => return Err(ConfigError::new(SHAPE)),
    };

    let mut errors = ConfigError::default();
    if url.host_str().is_none_or(str::is_empty) {
        errors.push("DATABASE_URL must name the host");
    }
    if url.username().is_empty() {
        errors.push("DATABASE_URL must name the user");
    }
    if url.password().is_none_or(str::is_empty) {
        errors.push("DATABASE_URL must include the password");
    }
    if url.path().trim_start_matches('/').is_empty() {
        errors.push("DATABASE_URL must name the database");
    }
    // sqlx applies these query parameters after the parts above, so they
    // would silently override them.
    if url.query_pairs().any(|(key, _)| {
        matches!(
            &*key,
            "user" | "password" | "dbname" | "host" | "hostaddr" | "port"
        )
    }) {
        errors.push(
            "DATABASE_URL must not set user, password, dbname, host, hostaddr or port as query parameters",
        );
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    // The parser's own message is not shown: it could echo the URL.
    PgConnectOptions::from_str(raw).map_err(|_| ConfigError::new(SHAPE))
}

impl fmt::Debug for DbConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DbConfig")
            .field("host", &self.connect_options.get_host())
            .field("port", &self.connect_options.get_port())
            .field("database", &self.connect_options.get_database())
            .field("max_connections", &self.max_connections)
            .field("allow_privileged_role", &self.allow_privileged_role)
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
        let (pool, connect) = config.into_pool_parts();
        assert_eq!(pool.get_max_connections(), 5);
        assert_eq!(connect.get_database(), Some("scanpay"));
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
    fn refuses_libpq_variables_even_when_empty() {
        let err = DbConfig::from_env(&env(&[
            ("DATABASE_URL", URL),
            ("PGPASSWORD", ""),
            ("PGAPPNAME", " "),
            ("PGPASSFILE", "/tmp/pass"),
        ]))
        .unwrap_err();
        assert_eq!(err.problems().len(), 3);
        assert!(err.problems()[0].starts_with("PGPASSFILE"));
    }

    #[test]
    fn url_must_spell_out_every_part() {
        const USER: &str = "DATABASE_URL must name the user";
        const PASSWORD: &str = "DATABASE_URL must include the password";
        const DATABASE: &str = "DATABASE_URL must name the database";
        const QUERY: &str = "DATABASE_URL must not set user, password, dbname, host, hostaddr or port as query parameters";
        let cases: [(&str, &[&str]); 7] = [
            // every missing part is reported at once
            ("postgres://127.0.0.1:5432", &[USER, PASSWORD, DATABASE]),
            ("postgres://scanpay@127.0.0.1/scanpay", &[PASSWORD]),
            ("postgres://scanpay:@127.0.0.1/scanpay", &[PASSWORD]),
            ("postgres://scanpay:pw@127.0.0.1", &[DATABASE]),
            ("postgres://scanpay:pw@127.0.0.1/", &[DATABASE]),
            (
                "postgres://scanpay:pw@127.0.0.1/scanpay?user=other",
                &[QUERY],
            ),
            (
                "postgres://scanpay@127.0.0.1/scanpay?dbname=other",
                &[PASSWORD, QUERY],
            ),
        ];
        for (url, expected) in cases {
            let err = DbConfig::from_env(&env(&[("DATABASE_URL", url)])).unwrap_err();
            assert_eq!(err.problems(), expected, "{url}");
        }
        let ok = "postgres://scanpay:pw@db.internal:6432/scanpay?sslmode=verify-full";
        assert!(DbConfig::from_env(&env(&[("DATABASE_URL", ok)])).is_ok());
    }

    #[test]
    fn unusable_value_is_not_treated_as_unset() {
        struct BadUnicode;
        impl Env for BadUnicode {
            fn var(&self, key: &str) -> Result<Option<String>, ConfigError> {
                match key {
                    "DATABASE_URL" => Ok(Some(URL.to_owned())),
                    "DATABASE_MAX_CONNECTIONS" => Err(ConfigError::new(format!(
                        "{key} is set but is not valid UTF-8"
                    ))),
                    _ => Ok(None),
                }
            }
            fn is_set(&self, key: &str) -> bool {
                matches!(key, "DATABASE_URL" | "DATABASE_MAX_CONNECTIONS")
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
    fn privileged_role_is_refused_unless_opted_in() {
        let config = DbConfig::from_env(&env(&[("DATABASE_URL", URL)])).unwrap();
        assert!(!config.allow_privileged_role());
        let config = DbConfig::from_env(&env(&[
            ("DATABASE_URL", URL),
            ("DATABASE_ALLOW_PRIVILEGED_ROLE", "true"),
        ]))
        .unwrap();
        assert!(config.allow_privileged_role());
        let err = DbConfig::from_env(&env(&[
            ("DATABASE_URL", URL),
            ("DATABASE_ALLOW_PRIVILEGED_ROLE", "yes"),
        ]))
        .unwrap_err();
        assert_eq!(
            err.problems(),
            ["DATABASE_ALLOW_PRIVILEGED_ROLE must be true or false"]
        );
    }

    #[test]
    fn pool_parts_carry_the_settings() {
        let config = DbConfig::from_env(&env(&[
            ("DATABASE_URL", URL),
            ("DATABASE_MAX_CONNECTIONS", "7"),
        ]))
        .unwrap();
        let (pool, connect) = config.into_pool_parts();
        assert_eq!(pool.get_max_connections(), 7);
        assert_eq!(connect.get_database(), Some("scanpay"));
    }

    #[test]
    fn max_connections_range() {
        for (value, ok) in [
            ("1", true),
            ("100", true),
            ("0", false),
            ("101", false),
            ("-1", false),
        ] {
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
