# db

Database startup shared by every binary: opening the pool, refusing a role that bypasses row-level security (unless `DATABASE_ALLOW_PRIVILEGED_ROLE=true`, local development only), and checking or applying the migrations embedded from `migrations/`.

Only `apps/migrate` applies migrations. The API and worker check that every embedded migration is applied, succeeded and unchanged, and refuse to start otherwise.

Like the other infra crates, it holds no business rules and owns no domain tables.
