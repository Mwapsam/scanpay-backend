# Migrations

One forward-only migration stream for the whole database, run by `apps/migrate`.

**No migrations yet.** Nothing is added here for a domain until that domain's chunk passes its freeze gate (06 §2A). As of 21 Oct 2026, no chunk is frozen.

## Rules

- File name: `<timestamp>_<domain>_<description>.sql`, for example `20261101120000_payments_intents.sql`.
- Each file touches one domain's schema only. No cross-schema foreign keys.
- The first migration creates the service roles and grants. Which roles is **Open**: 00 §5 lists five (`scanpay_owner`, `scanpay_ledger`, `scanpay_app`, `scanpay_worker`, `scanpay_ops_read`), while Baseline 2.0 and Organization 0.4 assume `<domain>_producer` / `<domain>_contract` roles from Shared 0.7 §2. Settle it with Shared 0.7 before writing that migration.
- `apps/api` refuses to start as a superuser or BYPASSRLS role unless `DATABASE_ALLOW_PRIVILEGED_ROLE=true`, which only local development (the compose superuser) sets. Once the service roles exist, local development should connect as them too and that setting can go.
- Forward-only: never edit a migration once merged; correct it with a new one.
- Every migration is reviewed by a second engineer and run against real PostgreSQL in CI.
- Ledger tables and posting functions wait for finance sign-off on the accounting model (00 §5).
