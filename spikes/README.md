# Spikes (throwaway)

Pre-freeze spikes (06 §2A): code that answers questions only a build can answer, run against a **disposable** database. Nothing here is production code, nothing is reused, and no schema here is the production schema. Spikes are outside the Cargo workspace.

When a spike's questions are answered, record the findings where they belong (the chunk draft, 05, or an ADR in `docs/adr/`), then delete the spike.

| Spike | Questions | Run |
| --- | --- | --- |
| `postgres/` | Tenant `set_config` scope; RLS through owner-privilege contract views; PostgreSQL 17 `transaction_timeout`; SQLSTATEs as seen through SQLx; lost-connection errors | `docker compose up -d`, then `cargo run --manifest-path spikes/postgres/Cargo.toml` |

Findings of `postgres/` (21 Oct 2026): [docs/findings/2026-10-21-postgres-spike.md](../docs/findings/2026-10-21-postgres-spike.md). The spike can be deleted once those findings are reflected in the chunk drafts.

`postgres/` creates a scratch database and a few uniquely named roles in the local compose cluster, then drops them. Never point it at a shared or production server.
