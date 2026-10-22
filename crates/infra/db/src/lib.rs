//! Database startup shared by every binary: open the pool, refuse a role that
//! bypasses row-level security, and check or apply migrations.
//!
//! Only `apps/migrate` applies migrations ([`apply_migrations`]); the API and
//! worker call [`check_schema`] and refuse to start if the database is behind.
//! This crate owns no domain tables and knows nothing about any domain.

use std::{collections::HashMap, process::ExitCode, time::Duration};

use scanpay_config::DbConfig;
use sqlx::{
    PgPool,
    migrate::{Migration, Migrator},
};
use tracing::{Instrument, error, info, info_span, warn};

/// Every migration in `migrations/`, embedded at build time, so a binary's
/// expected schema is fixed when it is built.
pub static MIGRATOR: Migrator = sqlx::migrate!("../../../migrations");

/// The forward migrations this build embeds; sqlx also embeds down
/// migrations, which are never run (the stream is forward-only).
fn up_migrations() -> impl Iterator<Item = &'static Migration> {
    MIGRATOR
        .iter()
        .filter(|m| !m.migration_type.is_down_migration())
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// A summary from [`describe_connect_error`], never the server's message:
    /// PostgreSQL's authentication errors name the user.
    #[error("could not connect to PostgreSQL: {0}")]
    Connect(String),
    #[error("database query failed during startup: {0}")]
    Query(#[source] sqlx::Error),
    #[error(
        "refusing to run as a superuser or BYPASSRLS role: row-level security would not apply \
         (set DATABASE_ALLOW_PRIVILEGED_ROLE=true only for local development)"
    )]
    PrivilegedRole,
    #[error("the database schema is behind this build: migration {0} is not applied (run migrate)")]
    MigrationMissing(i64),
    #[error("migration {0} failed part-way; repair it before starting")]
    MigrationFailed(i64),
    #[error("migration {0} was changed after it was applied; migrations are forward-only")]
    MigrationChanged(i64),
    #[error("applying migrations failed: {0}")]
    Migrate(#[source] sqlx::migrate::MigrateError),
}

/// Opens the pool (which opens and checks one connection) and checks the role
/// it connected as. Runs in a `db.connect` span recording host, port and
/// database only; never the user or password.
pub async fn connect(config: DbConfig) -> Result<PgPool, DbError> {
    let allow_privileged_role = config.allow_privileged_role();
    let (pool_options, connect_options) = config.into_pool_parts();
    let span = info_span!(
        "db.connect",
        host = connect_options.get_host(),
        port = connect_options.get_port(),
        database = connect_options.get_database().unwrap_or_default(),
    );
    async move {
        let pool = pool_options
            .connect_with(connect_options)
            .await
            .map_err(|err| DbError::Connect(describe_connect_error(&err)))?;
        if is_privileged_role(&pool).await.map_err(DbError::Query)? {
            if !allow_privileged_role {
                pool.close().await;
                return Err(DbError::PrivilegedRole);
            }
            warn!(
                "connected as a superuser or BYPASSRLS role (allowed by \
                 DATABASE_ALLOW_PRIVILEGED_ROLE): row-level security does not apply"
            );
        }
        info!("connected to PostgreSQL");
        Ok(pool)
    }
    .instrument(span)
    .await
}

/// How long [`with_pool`] waits for checked-out connections when closing. A
/// connection still held by a hung query is then dropped with the runtime,
/// and PostgreSQL rolls its transaction back.
pub const POOL_CLOSE_TIMEOUT: Duration = Duration::from_secs(3);

/// Connects, runs `work`, and closes the pool on every path. A connection
/// failure is logged and gives exit code 1. The binaries' shared skeleton,
/// so a new startup step is added once.
pub async fn with_pool(config: DbConfig, work: impl AsyncFnOnce(&PgPool) -> ExitCode) -> ExitCode {
    let pool = match connect(config).await {
        Ok(pool) => pool,
        Err(err) => {
            error!(error = %err, "database startup failed");
            return ExitCode::FAILURE;
        }
    };
    let code = work(&pool).await;
    if tokio::time::timeout(POOL_CLOSE_TIMEOUT, pool.close())
        .await
        .is_err()
    {
        warn!(
            timeout_secs = POOL_CLOSE_TIMEOUT.as_secs(),
            "connections still in use when the pool closed; dropping them"
        );
    }
    code
}

/// What went wrong, without the server's message (which names the user on
/// an authentication failure) or anything from the URL. A server refusal is
/// identified by its SQLSTATE (28P01 wrong password, 3D000 no such database,
/// 28000 no matching pg_hba.conf entry).
fn describe_connect_error(err: &sqlx::Error) -> String {
    match err {
        sqlx::Error::Database(db) => format!(
            "the server refused the connection (SQLSTATE {})",
            db.code().as_deref().unwrap_or("unknown")
        ),
        sqlx::Error::Io(io) => format!("network error: {io}"),
        sqlx::Error::Tls(_) => "TLS negotiation failed".to_owned(),
        sqlx::Error::PoolTimedOut => "timed out waiting for a connection".to_owned(),
        _ => "the connection could not be established".to_owned(),
    }
}

/// A superuser or BYPASSRLS role is not subject to row-level security, even
/// FORCE RLS (docs/findings/2026-10-21-postgres-spike.md), which would
/// silently defeat tenant isolation (I9, R8).
async fn is_privileged_role(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = current_user")
        .fetch_one(pool)
        .await
}

/// Refuses unless every migration this build embeds is applied, succeeded and
/// unchanged. A database with migrations this build does not know is newer
/// than the build (normal during a rolling deploy) and only warned about.
#[tracing::instrument(name = "db.check_schema", skip_all)]
pub async fn check_schema(pool: &PgPool) -> Result<(), DbError> {
    let applied = applied_migrations(pool).await.map_err(DbError::Query)?;
    let checked = compare(
        &applied,
        up_migrations().map(|m| (m.version, m.checksum.as_ref())),
    )?;
    info!(checked, "schema is current");
    Ok(())
}

/// Applies every pending migration. Only `apps/migrate` calls this.
#[tracing::instrument(name = "db.apply_migrations", skip_all)]
pub async fn apply_migrations(pool: &PgPool) -> Result<(), DbError> {
    MIGRATOR.run(pool).await.map_err(DbError::Migrate)?;
    info!(total = up_migrations().count(), "migrations applied");
    Ok(())
}

/// Applied migrations by version: (checksum, success). Empty when sqlx's
/// bookkeeping table does not exist yet.
async fn applied_migrations(pool: &PgPool) -> Result<HashMap<i64, (Vec<u8>, bool)>, sqlx::Error> {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await?;
    if !exists {
        return Ok(HashMap::new());
    }
    let rows: Vec<(i64, Vec<u8>, bool)> =
        sqlx::query_as("SELECT version, checksum, success FROM public._sqlx_migrations")
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(version, checksum, success)| (version, (checksum, success)))
        .collect())
}

/// The decision, separated from I/O so it can be tested without a database.
/// Returns how many embedded migrations were checked.
///
/// A failed migration refuses startup whoever wrote it: a newer build's
/// half-applied migration leaves the schema as unsafe as one of our own.
fn compare<'a>(
    applied: &HashMap<i64, (Vec<u8>, bool)>,
    expected: impl Iterator<Item = (i64, &'a [u8])>,
) -> Result<usize, DbError> {
    if let Some(version) = applied
        .iter()
        .filter(|(_, (_, success))| !success)
        .map(|(version, _)| *version)
        .min()
    {
        return Err(DbError::MigrationFailed(version));
    }
    let mut checked = 0;
    for (version, checksum) in expected {
        checked += 1;
        match applied.get(&version) {
            None => return Err(DbError::MigrationMissing(version)),
            Some((stored, _)) if stored.as_slice() != checksum => {
                return Err(DbError::MigrationChanged(version));
            }
            Some(_) => {}
        }
    }
    // Every checked version is in `applied`, so the rest are unknown to us.
    let newer = applied.len() - checked;
    if newer > 0 {
        warn!(
            newer,
            "the database has migrations this build does not know; it is newer than this build"
        );
    }
    Ok(checked)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(rows: &[(i64, &[u8], bool)]) -> HashMap<i64, (Vec<u8>, bool)> {
        rows.iter()
            .map(|(v, c, s)| (*v, (c.to_vec(), *s)))
            .collect()
    }

    #[test]
    fn empty_database_and_no_migrations_is_current() {
        assert!(compare(&applied(&[]), std::iter::empty()).is_ok());
    }

    #[test]
    fn missing_failed_and_changed_migrations_are_refused() {
        let expected = [(1, b"a".as_slice()), (2, b"b".as_slice())];
        assert!(matches!(
            compare(&applied(&[(1, b"a", true)]), expected.into_iter()),
            Err(DbError::MigrationMissing(2))
        ));
        assert!(matches!(
            compare(
                &applied(&[(1, b"a", true), (2, b"b", false)]),
                expected.into_iter()
            ),
            Err(DbError::MigrationFailed(2))
        ));
        assert!(matches!(
            compare(
                &applied(&[(1, b"x", true), (2, b"b", true)]),
                expected.into_iter()
            ),
            Err(DbError::MigrationChanged(1))
        ));
    }

    #[test]
    fn current_and_newer_databases_are_accepted() {
        let expected = [(1, b"a".as_slice())];
        assert!(matches!(
            compare(&applied(&[(1, b"a", true)]), expected.into_iter()),
            Ok(1)
        ));
        assert!(matches!(
            compare(
                &applied(&[(1, b"a", true), (9, b"z", true)]),
                expected.into_iter()
            ),
            Ok(1)
        ));
    }

    #[test]
    fn a_failed_migration_from_a_newer_build_is_refused() {
        let expected = [(1, b"a".as_slice())];
        assert!(matches!(
            compare(
                &applied(&[(1, b"a", true), (9, b"z", false)]),
                expected.into_iter()
            ),
            Err(DbError::MigrationFailed(9))
        ));
    }

    #[test]
    fn connect_errors_do_not_repeat_the_server_message() {
        let io = sqlx::Error::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "refused",
        ));
        assert_eq!(describe_connect_error(&io), "network error: refused");
        let protocol =
            sqlx::Error::Protocol("password authentication failed for user \"app\"".into());
        assert!(!describe_connect_error(&protocol).contains("app"));
    }

    #[test]
    fn this_build_embeds_no_migrations_yet() {
        // No chunk is frozen (06 §2A). This fails the day a migration is added,
        // as a prompt to check that its chunk has passed freeze gate 1.
        assert_eq!(MIGRATOR.iter().count(), 0);
    }
}
