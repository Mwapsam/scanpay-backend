# Spikes (throwaway)

Pre-freeze spikes (06 §2A): code that answers questions only a build can answer, run against a **disposable** database. Nothing here is production code, nothing is reused, and no schema here is the production schema. Spikes are outside the Cargo workspace.

When a spike's questions are answered, record what was observed in `docs/findings/`, carry the consequences into the chunk draft, 05 or an ADR in `docs/adr/`, then delete the spike (06 §2A).

| Spike | Questions | Run |
| --- | --- | --- |
| `postgres/` | Tenant `set_config` scope; RLS through owner-privilege contract views; PostgreSQL 17 `transaction_timeout`; SQLSTATEs as seen through SQLx; lost-connection errors | `docker compose up -d`, then `cargo run --manifest-path spikes/postgres/Cargo.toml` |

Findings of `postgres/` (21 Oct 2026): [docs/findings/2026-10-21-postgres-spike.md](../docs/findings/2026-10-21-postgres-spike.md). Not yet carried into the chunk drafts, so the spike stays for now.

`postgres/` creates a scratch database and a few uniquely named roles in the local compose cluster, then drops them. It refuses to run unless the `DATABASE_URL` host is `127.0.0.1`, `localhost` or `::1`; still, never point it at a shared server reached through a local tunnel.
