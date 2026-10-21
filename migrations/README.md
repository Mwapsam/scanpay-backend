# Migrations

One forward-only migration stream for the whole database, run by `apps/migrate`.

**No migrations yet.** Nothing is added here for a domain until that domain's chunk passes its freeze gate (06 §2A). As of 21 Oct 2026, no chunk is frozen.

## Rules

- File name: `<timestamp>_<domain>_<description>.sql`, for example `20261101120000_payments_intents.sql`.
- Each file touches one domain's schema only. No cross-schema foreign keys.
- The first migration creates the service roles and grants (04 §6).
- Forward-only: never edit a migration once merged; correct it with a new one.
- Every migration is reviewed by a second engineer and run against real PostgreSQL in CI.
- Ledger tables and posting functions wait for finance sign-off on the accounting model (00 §5).
