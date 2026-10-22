# Scanpay

Scanpay is a Zambia-first business operating system: customers and businesses chat, invoice, take payments and run their books in one place ("Pay. Connect. Grow."). Payments start with MTN Mobile Money; Airtel Money, Zamtel Kwacha and cards follow, each behind its own gate. This is a greenfield rebuild: no legacy code, schema or data is carried forward.

## Project status (read first)

- **Pre-development.** The specifications exist; the code is a Phase 0 skeleton: a `/health` API, validated startup config (`crates/config`), validated value types (`crates/types`: use these, don't write new money or phone types), telemetry startup (`crates/telemetry`), database startup (`crates/infra/db`), and empty domain and infra crates. No domain logic or tables exist. Nothing is verified yet. Local development runs as a PostgreSQL superuser, so R8 tenant isolation does not hold there (see Open decisions: database role layout).
- **No database changes before freeze.** Do not write production migrations or create tables for a domain until that domain's chunk has passed freeze gate 1 (06 §2A). As of 21 Oct 2026, no chunk is frozen. A throwaway spike may use a disposable database to answer PostgreSQL, SQLx or MTN-sandbox questions; its schema and code are not kept.
- **Building starts with the MVP**, the first build slice (06 §2A): shared, organization, minimum workflow, payments (MTN collection) and finance posting. It proves G0 on the simulated provider, then G1 on the MTN sandbox. It is blocked until shared 0.7 is frozen (CR-1 to CR-11) and payments has a full draft.
- **No real customer money before G5** (BoZ sandbox approval plus the two-approver initiation switch). Before then: simulated provider, MTN sandbox, or a closed internal trial if BoZ allows it. Whether such a trial may move real money is Open (06 §6); until decided, it does not.
- **Status words mean specific things.** Proposed = recommended, awaiting approval. Open = needs a named owner's decision. Accepted = approved. Verified = proven by passing tests and independently reviewed. Never mark anything Verified because code or docs exist.

## Documents

The six pre-development documents are in `docs/` (check that folder for exact filenames). Read the relevant one before changing behaviour in its area.

| # | Document | Governs |
| --- | --- | --- |
| 01 | Product Requirements Document | What the product must do: capability areas, requirement IDs (PAY, LED, USR, KYC, TEAM, ELG, APR, CNV, RFD, POS, REC, STS, SUP…), gates G0–G5 |
| 02 | Application Flow | Every journey step by step, including failure and recovery; the state and label mapping table |
| 03 | Design Brief | Brand, UX rules, payment status labels and copy, accessibility; points to the approved UI |
| 04 | Backend Schema | Data model by domain: payment and ledger tables, invariants I1–I11, module-to-domain mapping, design-level specs for each domain |
| 05 | Technical Requirements Document | Rust architecture, API and idempotency rules, provider integration, security, tenant isolation, operations, ADRs |
| 06 | Implementation Plan | Phase 0, architecture freeze gates, G0–G5 exit criteria, test matrix, decision and open-decision registers |

Related documents, held outside this repo (Claude Docs):

- **00 Product and Financial Integrity Contract** governs 01–06: payment states, commit boundary (T1–T4), provider failure matrix (F1–F15), evidence rules, invariants. Where a document disagrees with 00, 00 wins until changed.
- **Architecture Baseline 2.0**: the ten domains, ownership, events, permissions, full decision register (§15) and freeze gates (§16).
- **Organization Draft 0.4** (chunk 2) and **Workflow Draft 0.3** (chunk 4): the reference level of detail for every domain draft.
- **Shared draft 0.7** (chunk 1): identity, organizations, documents, audit, outbox. Not yet frozen; contract requests CR-1 to CR-11 are pending.

If two documents conflict, do not pick one silently. Point out the conflict and which document governs.

## Stack

- Backend: Rust modular monolith (Tokio, Axum, SQLx proposed). API and worker processes.
- Database: PostgreSQL, self-managed by Scanpay, one schema per domain.
- Mobile and tablet: React Native with Expo. Web business workspace and operations console: frameworks undecided (ADR-016).
- Hosting: Infratel Zambia (confirmation pending).
- Only the backend talks to MTN, SMS and KYB providers; clients never hold provider credentials.

## Repository layout

- `apps/api`, `apps/worker`, `apps/migrate`: the three binaries (05 §1). Keep handlers thin; logic lives in domain crates.
- `crates/domains/<domain>`: one crate per domain (`scanpay-<domain>`). `src/domain/` is pure logic with no I/O; `src/db/` touches only that domain's schema. Each README states the chunk, its status and blocking decisions; update it when the chunk's status changes.
- `crates/infra/<service>`: clients for external services (`mtn-momo`, `provider-sim`, `sms`, `storage`, `push`) and `db` (pool, role check, migrations). No business rules, no domain tables.
- `crates/types` (`scanpay-types`): validated value types — `Money`, `Currency`, `PhoneE164`, `IdempotencyKey`, `OrganizationId`, `new_id()`. Parse at the edge and pass these types inward; never raw strings or floats.
- `crates/config` (`scanpay-config`): startup configuration. `Env` is where values come from (the process, or a `HashMap` in tests); `DbConfig` is the database settings. No output of its own. Every problem is reported together; secrets never appear in messages, and `DbConfig`'s `Debug` shows host, port, database and pool size only, never the user or password.
- `crates/telemetry` (`scanpay-telemetry`): logging, span export and the startup sequence. It is the only place that decides where logs go; libraries emit events and never configure a subscriber.

## Startup sequence

Every binary's `main` is synchronous and does exactly this:

```rust
const SERVICE: Service = Service { name: "scanpay-api", version: env!("CARGO_PKG_VERSION") };
let (config, telemetry) = match scanpay_telemetry::start(SERVICE, ApiConfig::from_env) { … };
scanpay_telemetry::run(telemetry, run(config))
```

1. **Load `./.env`** (working directory only; parent directories are never searched). A missing file is normal. A malformed or unreadable one is a configuration problem: dotenvy stops at the bad line, so later variables would silently go unset. Its problem is held, not lost, until logging exists.
2. **Install logging once** from `RUST_LOG`, `LOG_FORMAT` and `OTEL_EXPORTER_OTLP_ENDPOINT`. There is no bootstrap logger and no reinstall: nothing is logged before this point, and `tracing` allows one global subscriber. Then log `starting` with the service name and version.
3. **Build config** (`Env` → `ApiConfig`, `DbConfig`).
4. **Any problem from 1–3** (including invalid logging settings): log each one as an `error`, exit 1.
5. **Async work**, inside `scanpay_telemetry::run`, which builds the runtime. Each binary wraps its work in `scanpay_db::with_pool`, which connects, runs it, and closes the pool on every path:
   - `api`: connect (refuse a superuser or BYPASSRLS role unless `DATABASE_ALLOW_PRIVILEGED_ROLE=true`) → check the schema is current → bind → serve.
   - `worker`: connect → check the schema → (jobs and the outbox, once they exist).
   - `migrate`: connect → apply forward-only migrations → exit.
6. **Shutdown:** Ctrl-C or SIGTERM stops new connections, in-flight requests finish, the pool closes, the runtime stops, and only then is telemetry dropped, flushing exported spans outside the runtime. Do not leave an outbox row half-dispatched on a normal shutdown. Every step is bounded, so the whole sequence fits inside the usual 30 s before SIGKILL: in-flight requests 15 s (then dropped), pool close 3 s, runtime stop 2 s, span flush 5 s. A dropped request's open transaction is rolled back by PostgreSQL; its outcome is unknown to the caller, who retries with the same idempotency key.

Migrations are applied only by `apps/migrate`. The API and worker refuse to start if any embedded migration is not applied, failed part-way, or was changed after it was applied; a database with migrations the build does not know is newer than the build and only warned about. `migrate` creates sqlx's bookkeeping table `public._sqlx_migrations` on first run; it is not a domain table. Build commit and time join `Service` once CI supplies them.

## Logging and tracing

- Output goes through `tracing` only; `print!`/`eprint!` are denied by workspace lints. Libraries emit events; only `scanpay-telemetry` decides where they go.
- Spans record chosen fields only (`#[instrument(skip_all)]` or `info_span!` with explicit fields). Never put secrets, tokens, OTPs, PINs or personal data (phone numbers, names) in fields or messages.
- HTTP requests carry an `x-request-id` on an `http.request` span (method, path, `request_id`); never log headers or bodies. Each request span is a trace root, so every request is its own trace; the process-long `service` span covers only startup, the binary's own work and shutdown. A caller's id is kept only if it is a single header of 1–128 characters from `[A-Za-z0-9._-]`; otherwise it is replaced with a new UUID, so a caller cannot inject newlines or bulk into every log line.
- Audit records go to the `shared` audit table, not to logs. Logs are not evidence. A future `security` target is for genuine security signals (authentication failures, role escalation, RLS violations), never a substitute for the audit table (R9).
- Database connection failures are logged by kind and SQLSTATE only (e.g. 28P01 wrong password), never the server's message, which names the user.
- Pure validators in `crates/types` are not instrumented: hot paths, and their inputs are personal data.
- OpenTelemetry (05 §1): spans are exported over OTLP/HTTP when `OTEL_EXPORTER_OTLP_ENDPOINT` is set (`http://` or `https://`), with `service.name` and `service.version`; nothing is exported otherwise. The value is trimmed and validated at startup and passed to the exporter explicitly (`<endpoint>/v1/traces`), so it never reads a different raw value; `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` is refused if set. Which collector is a deployment decision. Span fields leave the process when export is on, so the no-secrets, no-personal-data rule applies to them too.

### Defaults, filtering and formats

- **`RUST_LOG` default when unset:** `info,sqlx=warn,hyper=warn,tower=warn,tower_http=info`. `tower_http=info` is required, not redundant: directive targets match by plain string prefix, so `tower=warn` alone would also silence request logs (tested in `scanpay-telemetry`). Production must not default to `debug` or `trace`.
- **Targets are module paths.** Per-crate filtering is done with directives, e.g. `RUST_LOG=info,scanpay_payments=debug,scanpay_workflow=info`. No separate logger objects and no per-crate subscribers; target filtering replaces the "sub-logger" pattern. Pre-MVP no extra targets exist because no domain crate has logic yet; once they do, per-crate directives are the intended way to raise detail.
- **`LOG_FORMAT` values:** `text` (default when unset) or `json`. Production must set `LOG_FORMAT=json` explicitly. Any other value is a configuration problem: the binary logs it and exits 1, as for an invalid `RUST_LOG`.
- **No field values in messages that duplicate span fields.** Redaction is the emitter's responsibility; the subscriber does not redact.

### Trace and event conventions

- **Commands** emit `info!` with `command_key`, `aggregate_id`, `organization_id` (id only, not name) and `outcome`. Business refusals are `info!` with `refusal = <code>`; unexpected errors are `error!`.
- **State transitions** emit `info!` with `from`, `to`, and the aggregate id. No payloads.
- **Provider calls** are spans named `provider.<provider>.<operation>` carrying `provider`, `operation`, `idempotency_key`. No request or response bodies, no phone numbers, no MSISDNs.
- **Detailed flow inside a domain** uses `trace!` (or a `debug!` for branch-level decisions worth keeping at info+ in local runs). Hot paths do not emit per-item events at `info`.
- **HTTP request spans** are named `http.request` and carry `method`, `path` and `request_id` (the `x-request-id` value). The command span carries the command key, so logs join to the idempotency record.
- **Callback and polling paths** must not log per-attempt at `info`. Use `debug!`/`trace!` for retries and duplicate callbacks; R6 (harmless duplicates) is exactly the case where noisy logs would otherwise drown the signal.
- **Sampling/rate limiting:** none by default. If a path is too noisy, lower its target to `debug`; do not add a bespoke sampler.

## `spikes/`

- Throwaway pre-freeze spikes, outside the workspace, on disposable databases.
- Findings are recorded in `docs/findings/`, then carried into the chunk draft, 05 or an ADR; only then is the spike deleted (06 §2A).
- Spike code is never reused. The 21 Oct PostgreSQL findings are recorded but not yet carried over.

## `migrations/`

One forward-only stream, `<timestamp>_<domain>_<description>.sql`. Empty until a chunk is frozen.

## `docs/`

The six specifications; `docs/adr/` for ADRs; `docs/providers/` for written provider confirmations; `docs/runbooks/` for operational runbooks; `docs/findings/` for spike results.

## SQLx

The SQLx version is set by ADR. The current draft proposes 0.9. When that version is in use:

- 0.9 accepts only `&'static str` SQL.
- Wrapping a built string in `AssertSqlSafe` is a deliberate, reviewable exception, never a habit.
- Confirm the details against the SQLx release notes when the ADR is accepted; do not carry version-specific rules forward by memory.

A crate depends on another domain only through its published commands, events or contract views. Shared versions and lints live in the root `Cargo.toml`.

## Architecture rules

- **Ten domains:** shared, organization, workflow, messaging, commerce, payments, retail, finance, notifications, support. Reporting is read models, not a domain. Do not add domains such as communication, growth or services.
- **One writer:** each table belongs to one domain's schema and producer role. No domain writes another's tables. No cross-schema foreign keys; other domains are read only through named read-only contract views.
- **Events:** written to the outbox in the same transaction as the change; delivered at least once; consumers deduplicate by event id and order by aggregate version.
- **Approvals** live in `workflow`. An approval authorizes one exact command once; the owning domain rechecks and executes it. Approval never means money moved.
- **Tenant isolation:** `organization_id` on every business row; `SET LOCAL app.organization_id` after authorization; row-level security forced on every table; permission checked on the server for every command.

### `shared` vs `organization` boundary

`shared` draft 0.7 covers identity, organizations, documents, audit and outbox. The `organization` domain covers business profile and settings. The `organizations` entity and its membership are written by `shared`; the `organization` domain reads them through a `shared` contract view. Do not create a second organization entity, and do not write `shared` tables from `organization` or vice versa. The precise split for staff roles is settled in the chunk drafts; until then, keep new columns in the domain that already owns the table.

## Financial invariants (non-negotiable)

- **R1 Silence never settles.** A timeout, missing callback or unknown answer is never paid or failed. A callback is a hint; only a provider query (or an approved operator resolution) decides.
- **R2 One payment per thing.** One open payment request per invoice; one electronic tender per sale; idempotency key stored before every provider call; a retry inquires, never resubmits.
- **R3 Nothing is edited.** Corrections are new records: reversing journals, adjustments, refunds. Issued invoices are never edited.
- **R4 Approval is not settlement.**
- **R5 Finance posts only verified amounts**, from `payment.verified`, `refund.verified` and `payment.reversed`, once per source event.
- **R6 Late and duplicate provider answers are harmless.**
- **R7 Two people for sensitive actions;** nobody approves their own request.
- **R8 Organization isolation** on every read and write.
- **R9 Sensitive actions are audited** with actor, session, reason and outcome.
- **R10 Collection fails closed:** no eligibility decision, an active restriction, or no defined limit policy means no payment request.

> **Vocabulary note (N13 is Open).** `payment.verified`, `refund.verified` and `payment.reversed` are placeholder event names used to state R5. The final state and event vocabulary is decided by N13; when it lands, rename these in code, docs and contract views together. Do not use the placeholder names as evidence that the refund-vs-reversal question is settled.

## Coding conventions

- Money: integer minor units (`BIGINT`) plus `CHAR(3)` currency. Never floating point.
- Time: `timestamptz` in UTC; display in Africa/Lusaka. In a command, capture one evaluation instant with `clock_timestamp()` after taking locks and use it for every time check. Never use `now()` in a check: it is fixed at transaction start.
- Commands: insert the command key first, then a savepoint. Business refusals roll back to the savepoint and are stored, so a retry returns the same refusal. Serialization failures, deadlocks, timeouts and connection loss roll back the whole transaction (Organization 0.4 §3.8).
- A lost connection is an unknown outcome. Check the command key; never assume rollback.
- State machines are Rust enums with exhaustive transitions, backed by database triggers that reject illegal transitions.
- Time-based policy (eligibility, restrictions, limits, settlement accounts) is a series of immutable start-dated rows; scheduled rows can be cancelled before they start; rows never close or restore one another.
- Tests that involve concurrency, roles or transactions run against real PostgreSQL with the real roles, not mocks.
- Config tests pass values through `Env` (a `HashMap`), never by changing the process environment: `std::env::set_var` is `unsafe` in edition 2024 (and `unsafe_code` is forbidden) and races between parallel tests. sqlx still reads `PG*` from the real environment while parsing a URL, but the URL must spell out every value the tests assert on and the URL always wins, so outcomes do not depend on the shell.

## Open decisions: do not assume an answer

These are Open until a named owner signs them off (full register: Baseline 2.0 §15, 06 §6). Code and docs must keep them visible, not settle them by implementation.

- **N13** One vocabulary for payment states and for refund vs reversal. Until decided, use 00's state names (`uncertain`, `under_review`, `succeeded`…) in code and APIs; 02 has the mapping to UI labels. Event names in R5 are placeholders (see note above).
- **N1** Who approves refunds in a business with a single Owner. Until decided, such requests cannot be created.
- **Database role layout.** 00 §5 lists five roles (`scanpay_owner`, `scanpay_ledger`, `scanpay_app`, `scanpay_worker`, `scanpay_ops_read`); Baseline 2.0 and Organization 0.4 assume `<domain>_producer` / `<domain>_contract` roles from Shared 0.7 §2. Owner: engineering lead with security, when Shared 0.7 is frozen. Until then local development connects as the compose superuser with `DATABASE_ALLOW_PRIVILEGED_ROLE=true`, and **R8/I9 do not hold at runtime there**: row-level security does not apply to a superuser. Any other environment refuses such a role.
- **Closed internal trial before G5:** whether it may move real money, whose, and under which BoZ condition (06 §6). Owner: product with compliance. Until decided, no trial moves real money.
- **DEC-31** business and staff roles · **DEC-34** collecting during KYB review — safe default is **off** until decided · **DEC-35** limit values · **DEC-23** approval thresholds and expiry · **DEC-37** till and cash rules · **DEC-01 / DEC-18** chart of accounts and whether Scanpay holds funds · **C2 / C3** operator resolution, statement lines as evidence · **DEC-27** direct MTN vs aggregator · MTN refund API support.

## Working in this repo

- Before implementing a feature, find its requirement ID in 01, its journey in 02, its tables in 04 and its controls in 05, and check that its chunk is frozen (06 §2A).
- When something needs a decision that isn't recorded, stop and ask, or mark it Open with a recommended option. Don't invent product, finance, compliance or regulatory answers.
- Don't invent provider behaviour. MTN Zambia's contract is confirmed in writing, not assumed from generic MoMo documentation.
- Keep docs and code in step: if a change alters a contract, state machine or table, the matching document needs the same change.