# Scanpay 05 — Technical Requirements Document

Oct 18, 2026 · @TCS

## 1. Architecture and Rust stack

Scanpay is rebuilt as a single Rust modular monolith on PostgreSQL, deployed as two process types (API and workers); this is the simplest design that can guarantee payment correctness, and modules can be split out later if measurements justify it.

**Governed by 00.** The commit boundary (T1–T4), provider failure matrix (F1–F15), state-and-evidence matrix (00 §2.9), stop rules and invariants are defined in the 00 Product and Financial Integrity Contract; this document specifies how the Rust system implements and operates them. Scanpay is multi-provider (00 GD-1) and the pilot is planned within the BoZ Regulatory Sandbox, subject to acceptance (00 GD-2). This is a greenfield build: Rust backend, React Native (Expo) app and Infratel Zambia hosting, with no legacy code, schema or data carried forward (00 GD-3, GD-4).

*Business summary.* One well-structured system is cheaper to build, test and operate than many small services, and it keeps every payment change in one database transaction. Rust is chosen for safety and efficiency; correctness comes from the design, not the language alone.

**Version 1.2 (21 Oct 2026).** The domain architecture is now defined in [Architecture Baseline 2.0](https://claude.ai/artifact/3TaqDBgNAXW9Y8P6cUW3xS), with two domains detailed in [Organization Draft 0.4](https://claude.ai/artifact/K1jPBwUKMcLFrJxCxbaoRF) and [Workflow Draft 0.3](https://claude.ai/artifact/7Re3AyWYUhyf6Z4fjAbE72). This TRD now points to them rather than restating them. Main changes: ten domains with one PostgreSQL schema and one writer each (§7); tenant context, contract views and the evaluation-instant rule (§4); per-domain command keys replacing one platform idempotency table (§2); approvals as their own domain (§7); text conversations moved to G1 (§5); PostgreSQL self-managed by Scanpay (§6). Everything remains Proposed unless marked Decided.

&#91;embedded content: proposed runtime architecture · G1–G3\]

Clients reach the API through the edge; workers make the slow and retryable calls to MTN; every component writes state only through PostgreSQL. Realtime messaging (G2) and calling (G4) add components described in §5.

### Proposed stack (each an ADR to confirm)

| Concern | Proposed choice | Why | Alternative considered |
| --- | --- | --- | --- |
| Async runtime | Tokio | De facto standard; mature ecosystem | async-std (smaller ecosystem) |
| HTTP framework | Axum | Tokio-native, tower middleware, typed extractors | Actix Web |
| Database access | SQLx (compile-time checked SQL) | Explicit SQL for ledger work; no ORM magic in money paths | SeaORM, Diesel |
| Migrations | sqlx-migrate, forward-only, reviewed | Reproducible schema history | Refinery |
| Serialization | serde / serde\_json | Standard | — |
| Outbound HTTP | reqwest with rustls | Timeouts, TLS without OpenSSL | hyper directly |
| Password/passcode hashing | argon2 (Argon2id) | Current recommended KDF | bcrypt |
| Observability | tracing + OpenTelemetry | Structured logs, traces, metrics | log + custom |
| Background jobs | Postgres-backed queue (shared 0.7 jobs, `SKIP LOCKED`) | No extra infrastructure; transactional enqueue | Redis-based queue |
| Config and secrets | Environment + managed secret store; no secrets in repo | Rotation, audit | — |
| Mobile client | React Native with Expo, Android and iOS, built new (decided) | One codebase for both platforms; typed client generated from the OpenAPI contract; Expo build and update tooling | — |
| Ops console | Separate web app with its own sign-in; framework open. The business web workspace is a second web app; its framework is also open | Internal users only | — |

### Code structure

- Cargo workspace with one crate per Baseline 2.0 domain (shared, organization, workflow, messaging, commerce, payments, retail, finance, notifications, support), each split into pure domain logic and its own database access; infra crates for MTN, SMS, storage and push; `apps/api`, `apps/worker`, `apps/migrate`. A domain crate calls another domain only through that domain's published commands, events or contract views, never its tables.
- Domain crates expose commands and queries; handlers stay thin.
- State machines are Rust enums with exhaustive transition functions; illegal transitions don't compile or return typed errors.
- `#![forbid(unsafe_code)]` in domain crates; `clippy -D warnings` in CI.

## 2. API design, idempotency and versioning

Every state-changing request is idempotent and every response states the true current state; clients never infer outcomes.

### Conventions

- JSON over HTTPS; resource-oriented REST under `/v1`. Breaking changes require `/v2`; additive changes don't.
- OpenAPI specification generated from code and published to the client teams; contract tests in CI.
- Errors use a stable shape: `{ "code": "merchant_inactive", "message": "…", "request_id": "…" }`. Codes are documented; messages are not parsed by clients.
- Pagination by cursor, not offset.
- All money as `{ "amount_minor": 125000, "currency": "ZMW" }`.

### Idempotency (PAY-10, PAY-07)

The canonical rule is in 00 §4; every money-moving endpoint implements it the same way.

1. Client sends `Idempotency-Key` (UUID) on every POST that creates or moves something; the app stores it with the pending action so a restart reuses it.
2. Each domain stores the key, a versioned request fingerprint and the outcome in its own `command_keys` table, written first in the same transaction as the effect (Organization 0.4 §3.8 is the reference). A business refusal is stored, so a retry returns the same refusal; an infrastructure failure rolls back everything, key included, so a retry runs afresh.
3. Same key, same fingerprint, finished → the original response with the operation's **current** state.
4. Same key, same fingerprint, still in progress → `202 Accepted` with the operation ID and a status URL; the client polls that URL. No second operation is ever created.
5. Same key, different fingerprint → `409 idempotency_mismatch`.
6. A retry after an uncertain result resumes the existing operation; it never creates a new intent or provider request.
7. Key retention covers the longest retry, reconciliation and recovery window; the period is open (N12), and a compact key-and-fingerprint record may be kept after the full record is archived so a late retry never creates a new request.

### Core G1 endpoints (illustrative)

| Method and path | Purpose | Notes |
| --- | --- | --- |
| `POST /v1/auth/otp` · `POST /v1/auth/otp/verify` | Sign-up / new device | Rate-limited per phone and IP |
| `POST /v1/sessions/refresh` · `DELETE /v1/sessions/{id}` | Session lifecycle | Refresh token rotation |
| `GET /v1/merchants/resolve?qr=…` | Resolve QR to verified merchant | Returns display identity only |
| `POST /v1/payments` | Create payment intent | Idempotent; returns intent with state |
| `GET /v1/payments/{id}` | Current state and timeline | Source of truth for the app |
| `GET /v1/payments?cursor=…` | History | Payer or business scope by token |
| `POST /v1/payments/{id}/disputes` | Report a problem | Opens ops case |
| `POST /v1/providers/{provider}/callbacks` | Provider callback receiver | Authenticated per §3; always 200 after durable store |
| `POST /v1/businesses` · `/kyc/...` | Merchant onboarding and KYC | Uploads via pre-signed URLs |
| `GET /v1/businesses/{id}/payments/stream` | Counter-mode live feed | SSE or WebSocket; only `succeeded` |
| POST /v1/businesses/{id}/conversations/{cid}/payment-requests | Request payment from a conversation (PAY-20, PAY-21) | Idempotent; checks permission, eligibility, restrictions, limits; optional invoice\_id |
| GET /v1/businesses/{id}/conversations · /messages | Conversations and record cards | Card state read from payments, never from the message |
| POST /v1/businesses/{id}/invitations · POST /v1/invitations/accept | Team invitations (TEAM-03) | Accept requires the caller's verified phone to match |
| GET /v1/approvals · POST /v1/approvals/{id}/decisions | Approver inbox and decisions (APR) | Step-up evidence attached where the policy requires it |
| Ops: /ops/v1/organizations/{id}/eligibility · /restrictions · /limits | Staff proposals (ELG) | Create an approval request; the change is written only after a second staff member approves |

### Authorization

- Every handler resolves an `Actor` (user, business membership and role, or staff grant) and checks the permission catalogue (Baseline 2.0 §11) per action, reading the member's current role on each request. Actions that need approval create a workflow request instead of executing (§7).
- Business-scoped queries always filter by `organization_id` from the authenticated membership, never from the request body (USR-05).
- Ops endpoints live on a separate host, require SSO + MFA, and are audit-logged.

## 3. Payment provider integration (multi-provider; MTN first)

A callback is treated as a hint, not proof: Scanpay acts only on an outcome it has confirmed by querying MTN directly.

**Assumption to verify before G0 exit:** the description below follows the general shape of MTN's MoMo Open API (collection request-to-pay with a client-supplied reference ID, a status query by that ID, an optional callback URL, and a separate disbursement product). The current version, Zambia-specific behaviour, callback authentication, rate limits and sandbox fidelity must be confirmed from MTN documentation and our commercial agreement.

Generic MoMo documentation is not assumed to match MTN Zambia's current contract or sandbox behaviour; Phase 0 confirms them in writing. The same confirmation is required for every later provider before its adapter is built.

### Provider adapter

- A `PaymentProvider` Rust trait: `request_collection`, `query_collection`, `request_disbursement`, `query_disbursement`, `parse_callback`. MTN is the first implementation; a fake provider with scripted failures is used in tests (06 §5).
- The client-generated provider reference (UUID) is stored on the intent **before** the outbound call, so a timeout can always be queried.
- Outbound calls have explicit connect and total timeouts; retries only on safe operations (queries), never by re-sending a collection request with a new reference.
- Provider credentials and API keys live in the secret store, are rotated on a schedule, and are never logged.

### Providers and integration model (GD-1)

| Adapter | Method | Gate | Notes |
| --- | --- | --- | --- |
| `mtn_momo` | Mobile money | G1 | First implementation |
| `airtel_money` | Mobile money | G1+ sub-gate | Contract, sandbox and statements to confirm |
| `zamtel_kwacha` | Mobile money | G1+ sub-gate | Contract, sandbox and statements to confirm |
| `card_{partner}` | Cards | G1+ sub-gate | Acquirer or processor not selected; prefer hosted fields or tokenization so card data never touches Scanpay servers; card-data compliance scope confirmed with the partner |

**Integration model (ADR-010, open):** direct integration with each provider, or one aggregator offering several methods. Instacom Payment Solutions was previously considered; it is not selected and its capabilities are unverified. If an aggregator is chosen, it is one adapter with per-method behaviour documented; Scanpay still owns the ledger, payment lifecycle, evidence and reconciliation.

**Adapter certification (per provider, before enablement):** written contract confirmed; capability matrix completed and F1–F15 (00 §4) pass against the provider sandbox or documented compensating controls; stop rules recorded; statement import and reconciliation proven; runbook written. A provider is enabled per environment and user group by configuration only after this passes.

Each adapter reads its versioned capability declaration (00 §4 matrix; `payments.provider_capabilities` in 04) and applies that provider's ambiguous-dispatch rule (F13). A shared trait does not imply shared guarantees. For cards, the partner's authorization and capture model, voids, refunds, chargebacks, 3-D Secure, tokenization and PCI DSS scope are decided before the card sub-gate opens.

### Collection sequence

1. **T1** — intent → `submitting`, request reference saved, attempt `prepared`. Commit.
2. Call the provider with that reference (outside any transaction).
3. **T2** — acknowledged → `pending_customer`, `next_check_at` set. Documented rejection → `failed`. Timeout or unknown → `uncertain`, query job scheduled.
4. Callback arrives → store raw callback, return 200, enqueue verification. The callback is a hint.
5. Verification job queries the provider by the same reference. A definite outcome is written as immutable evidence (**T3**).
6. **T4** — apply evidence: state change, journal entry, projections, outbox, all at once (00 §3). Still pending → reschedule with backoff.
7. Stop rules reached without a definite outcome → `under_review` with an ops case. Automation never ends by marking a payment failed.

### Callback security

- An unguessable URL path and TLS are hygiene, not authentication. Callbacks must arrive through the edge over TLS.
- If MTN offers signatures, mutual TLS or IP allow-listing, all available mechanisms are enabled.
- Regardless, a callback alone never changes a financial state (PAY-12).
- Intake follows 00 §4 (Callback intake): authenticated callbacks are stored, then answered 2xx; storage failure returns 5xx; unauthenticated callbacks are quarantined, answered 401 and never trigger queries.

### Payouts (disbursements)

Same pattern: reserve in the ledger, store reference, call, verify by query, then confirm or release. A payout is never `paid` without a verified MTN success.

### Reconciliation

Daily import of MTN statements or transaction reports (format to confirm), matched on provider reference and amount; mismatches open reconciliation cases in the finance domain (02 §4). A statement line is evidence; it never itself changes a payment's state (C3 Open).

## 4. Security, privacy and compliance

G0 cannot exit until every item in this section marked G0 is implemented and verified; security is a release gate, not a later hardening phase.

### Controls

| Area | Requirement | Gate |
| --- | --- | --- |
| Secrets | All credentials in a managed secret store; existing secrets assumed exposed and rotated during rebuild; secret scanning in CI | G0 |
| Transport | TLS 1.2+ everywhere, HSTS; internal traffic encrypted | G0 |
| Data at rest | Database and object storage encrypted; KYC documents with separate keys and access logging | G0 |
| Authentication | OTP for registration/new device; passcode (Argon2id) or device biometric key; short-lived access tokens + rotating refresh tokens bound to device | G0 |
| Step-up | Re-authentication for payout destination changes, large payments, role changes | G1 |
| Authorization | Policy check per action; tenant filter from membership; tested by cross-tenant test suite | G0 |
| Rate limiting | Per IP, per phone, per user on auth, OTP, payments and resolve endpoints | G0 |
| Input handling | Strict typed deserialization; size limits; file type and malware checks on uploads | G0 |
| Logging | No PINs, OTPs, tokens, full documents or secrets in logs; PII masked | G0 |
| Audit | Append-only audit log for privileged and financial actions (04 §5) | G0 |
| Admin access | SSO + MFA, least privilege, dual approval for money-moving actions | G0 |
| Dependencies | `cargo audit` / `cargo deny` in CI; pinned lockfiles; SBOM per release | G0 |
| Testing | Threat model for payments; external penetration test before pilot | G5 |
| Credentials and recovery | Passcode (Argon2id) or passkey/device key; biometrics only unlock the device key and are never sent or stored; OTP for registration and new devices; recovery requires verified number plus a support check and revokes all sessions | G0 |
| Ownership and payout changes | Owner step-up plus independent ops approval; payouts to a new destination held until approved | G1 |
| Callbacks | Every authentication mechanism the provider offers is enabled; callbacks never change financial state without query-verified evidence | G0 |

### Privacy and data protection

- Data catalogue per column: purpose, sensitivity, retention class, deletion rule.
- Minimum collection: no contact-list upload without explicit opt-in; no location tracking.
- Account closure: personal data deleted or anonymised per retention class; financial records kept for the legally required period.
- **Obligations to validate (owner: legal/compliance):** Zambia's Data Protection Act (2021) registration and data-subject rights; Bank of Zambia payment-systems requirements for Scanpay's role; AML/CFT obligations for business onboarding; MTN merchant and aggregator terms; ZRA requirements before any Smart Invoice integration. This document does not assert which apply.

### Authentication and account-recovery protocol (P0-6)

The values below are proposals to confirm with security before G0 passes; the structure is required.

| Area | Requirement |
| --- | --- |
| Credential model | One device-bound key pair per installed app, created at enrollment and held in the platform keystore. Biometrics or the app passcode only unlock it locally; the server stores the public key, never biometric data. A passcode is a local unlock factor, not a server password. |
| Enrollment | Phone OTP (single use, short expiry, e.g. 5 minutes; at most 5 attempts) → device key registered → session issued. A new device triggers a security notification to existing devices. |
| Sign-in | Server sends a random challenge; the device signs it with its key; the signature is bound to challenge, device ID and expiry. |
| Sessions | Short-lived access token; refresh tokens bound to the device and rotated on every use. Reuse of a spent refresh token revokes the whole session family (replay detection). |
| Step-up | Fresh challenge signature with user presence, enforced server-side, for payments above a threshold, payout destination changes, role changes, new-device approval and recovery. |
| Lockout | Progressive delays on OTP and challenge failures; temporary lock after repeated failures; notification to the user. |
| Device revocation | User (from another device) or support can revoke any device; its keys and sessions are invalidated immediately. |
| Recovery | Lost device: OTP to the registered number plus a waiting period (e.g. 24–72 hours) with notification, before payouts and destination changes unlock. During the wait, existing payout destinations are frozen. |
| SIM-swap / takeover | A number change or recent SIM swap (if a carrier signal is available) forces the waiting period and freezes payouts. Merchant owners recovering an account need ops verification against KYC records. |
| Support role | Support can start recovery and revoke devices. Support can never set credentials, skip the waiting period or change a payout destination without second-person approval; every action is audit-logged. |
| Logout / uninstall | Local keys are deleted and the device is revoked server-side on logout; tokens left on an uninstalled device expire and cannot be refreshed. |

### BoZ sandbox controls (GD-2)

The exact controls depend on the conditions BoZ approves; the system is built so they are configuration, not code changes.

- **Limits and eligibility:** defined in the organization domain (limit definitions, limit requirements, eligibility decisions; Organization 0.4 §4) and enforced by payments, which reserves limit capacity atomically when a request is created; changes need a second staff approver and are audit-logged.
- **Disclosures and consent:** versioned disclosure text served by the API; consent recorded with version and timestamp.
- **Reporting:** reports BoZ requires generated from source data, stored immutably with query and data hash.
- **Monitoring:** dashboards for pilot volume against approved limits; alerts before a limit is approached.
- **Access:** pilot configuration changes restricted to named roles with MFA.
- **Deployment:** pilot cohort, enabled providers and limits set per environment through reviewed configuration; nothing outside the approved scope is enabled in production.
- **Exit:** the system can stop new pilot activity while still serving history, receipts, refunds and complaints.
- **Initiation switch:** a server-side flag, off by default in every environment, gates customer transaction initiation. Turning it on in production needs two named approvers after G5 (00 §8).

### Database access and tenant context (P0-2)

- Services connect as `scanpay_app` or `scanpay_worker` and cannot write ledger tables directly; all money moves through owner-role transition and posting functions (00 §5, 04 §3).
- Each domain has its own schema and producer role and writes only its own tables. Each request transaction sets `SET LOCAL app.organization_id` (and `app.user_id` where needed) after authorization, never from a request field. Row-level security, forced on every base table, reads these settings. Other domains read only through owner-privilege contract views and functions (Organization 0.4 §7.2). Limit stated plainly: the database cannot authenticate these settings, so row-level security protects against application mistakes, not compromised backend code (Workflow 0.3 §2.1).
- Adversarial integration tests run as the real service roles in CI against real PostgreSQL.

**Time and failure rules (from Baseline 2.0 drafts):**

- Every command captures one evaluation instant with `clock_timestamp()` after taking its locks and uses it for every time check; `now()`, which is fixed at transaction start, is never used in a check (Workflow 0.3 §2.1).
- Errors are classified: business refusals are stored after rolling back to a savepoint; serialization failures, deadlocks, timeouts and connection loss roll back the whole transaction (Organization 0.4 §3.8).
- Where a transaction duration bound is used (Workflow 0.3 §6.0): PostgreSQL 17's `transaction_timeout` ends the whole session, can fire mid-statement, and when set with `SET LOCAL` mid-transaction counts from the `SET`, not from `BEGIN` (observed on 17.10, 21 Oct 2026). Clients treat the resulting connection loss as an unknown outcome and check the command key.

### Mobile app security (React Native with Expo)

- Tokens and device keys are kept in the platform keystore (iOS Keychain, Android Keystore) via secure storage, never in plain async storage.
- Biometrics only unlock the device key; no biometric data leaves the device.
- Certificate pinning is evaluated for the API host; it must allow key rotation.
- Over-the-air updates are signed and restricted to UI changes; payment rules live on the server, so an app update can never change financial behaviour.
- Release builds disable debugging and strip logs that could contain personal data.

## 5. Real-time messaging and calling

Messaging ships in two steps — text with record cards over WebSocket at G1, because the first slice requests payment from a conversation; media and calls at G4 — and the G4 build starts only after a cost estimate and a provider decision.

### G1: text messaging and record cards

- WebSocket gateway inside the API process (Axum WebSocket), authenticated with the session token; one connection per device.
- Messages persisted first (`messaging.messages`), then fanned out to online members; offline members get push.
- Fan-out across multiple API instances via PostgreSQL `LISTEN/NOTIFY` initially; a dedicated broker (e.g. NATS or Redis) only when measurements require it.
- Client message IDs make sends idempotent; delivery and read receipts are separate events.

Record cards (invoice, payment, receipt) are messaging projections keyed by the source record; a card applies an event only if its source version is newer, and refreshes in-between payment states from payments' contract view. A card never shows paid before `payment.verified`.

### G4: voice notes, calls and conferences

- Voice notes: client uploads to object storage via pre-signed URL; message references the media ID. Format and maximum duration to decide (open).
- One-to-one audio/video: WebRTC, signaling over the existing WebSocket, STUN plus TURN relay for restrictive mobile networks.
- Conferences: a selective forwarding unit (SFU) is required; peer-to-peer mesh does not scale past a few participants.
- **Build vs buy (open decision):** self-host (e.g. coturn + a Rust or Go SFU such as LiveKit) vs a managed real-time provider. Decide on data residency, cost per participant-minute, Zambian network performance and team capacity.
- Calls are not recorded by default; any recording requires explicit consent and a retention policy.
- Call quality metrics (setup time, drop rate, relay usage) collected from day one of the G4 beta.

## 6. Performance, observability, operations and ADRs

Targets are set from measurement during G0–G1, not guessed; the first job is to measure baseline and MTN latency separately.

### Performance approach

- SLOs defined per endpoint class (payment create, payment status, history, resolve) with p95/p99 targets agreed after baseline measurement.
- MTN latency tracked as its own metric; Scanpay's own processing time reported separately.
- Load tests (06 §5) at an agreed multiple of expected pilot peak; capacity plan written from results.
- Database: connection pooling, query plans reviewed for every hot query, no N+1 in list endpoints.

### Observability

- Every request and job carries a request ID and, where relevant, payment ID and provider reference in its trace.
- Dashboards: payment funnel by state, age of `uncertain` / `under_review`, callback and enquiry outcomes, reconciliation breaks, error rates, latency.
- Alerts: any ledger invariant failure (page immediately), uncertain payments older than threshold, reconciliation breaks, MTN error spikes, queue backlog.

### Operations

- Environments: local, CI, staging (MTN sandbox), production. Infrastructure as code.
- Hosting: Infratel Zambia (selected; confirmation of data locations, network and physical security and the disaster-recovery site still pending). PostgreSQL is self-managed by Scanpay (decided 21 Oct 2026): Scanpay installs and runs it, including the btree\_gist extension, WAL archiving, point-in-time recovery and backups. Set RPO and RTO targets and prove them with a restore drill.
- Backups: PITR enabled; restore drill before pilot; RPO/RTO targets set in 06.
- Deploys: CI builds a reproducible container; migrations run before rollout; feature flags for gated features; blue-green or rolling deploy.
- Runbooks: uncertain payment backlog, MTN outage, reconciliation break, credential rotation, incident communication.

### Durable jobs and outbox

- Jobs are claimed with `SELECT … FOR UPDATE SKIP LOCKED` and a lease; a crashed worker's lease expires and the job is reclaimed.
- Each job kind has `max_attempts` and backoff; exhausted jobs are dead-lettered and alerted, never dropped.
- Workers stop claiming on shutdown signal and finish or release in-flight jobs within a grace period.
- Outbox events are published at least once; every consumer deduplicates by event ID (00 R6).
- CI runs integration tests against real PostgreSQL; backup restoration is drilled on a schedule and before the pilot.
- Every job claim carries a `lease_version` fencing token, and every write checks it plus the expected state (00 §3). A stale worker's writes are rejected; it can never create a new provider operation.

## 7. Domain architecture (Baseline 2.0)

This section summarizes the rules; Baseline 2.0 §5–§13 is the full specification.

| Rule | Requirement |
| --- | --- |
| Domains | Ten: shared, organization, workflow, messaging, commerce, payments, retail, finance, notifications, support. Reporting and exports are read models, not a domain |
| One writer | Each authoritative record has one owning domain; no domain writes another's schema; no cross-schema foreign keys |
| Events | Written to the shared outbox in the same transaction as the change; delivered at least once; consumers deduplicate by event id and order by aggregate version (Baseline §10) |
| Approvals | Owned by workflow. An approval authorizes one exact command; the owning domain rechecks and executes it and emits its own event; approval never changes money state (Workflow 0.3 §6) |
| Collection gates | Payments checks eligibility, restrictions and limits when a request is created and again before each provider call, failing closed (Baseline §12.1) |
| Limit reservation | Capacity is reserved in the same transaction that creates the payment request, under a lock on the limit counters, and held until the payment reaches a final state |
| Time-based policy | Eligibility, restrictions, limits and settlement accounts are immutable start-dated series; scheduled rows can be cancelled before they start (Organization 0.4 §4.0) |
| Status vocabulary | Being reconciled between 00, 02 and Baseline 2.0 (N13); APIs expose 00's state names until decided |

Domain specifications: organization and workflow are drafted; the remaining domains are specified in 04.

### Architecture decision records to confirm

| ADR | Decision | Status |
| --- | --- | --- |
| ADR-001 | Rust backend | Decided |
| ADR-002 | Modular monolith, two process types | Proposed |
| ADR-003 | PostgreSQL as single source of truth; Postgres-backed job queue | Proposed |
| ADR-004 | Tokio + Axum + SQLx | Proposed |
| ADR-005 | Double-entry append-only ledger (04 §3) | Proposed |
| ADR-006 | Callback-as-hint, verify-by-query | Proposed |
| ADR-007 | Mobile client approach | Decided — React Native with Expo |
| ADR-008 | Hosting provider and region | Selected — Infratel Zambia, confirmation pending |
| ADR-009 | Real-time media build vs buy | Open (before G4) |
| ADR-010 | Provider integration model: direct vs aggregator | Open (Phase 0) |
| ADR-011 | Card acquirer or processor and card-data scope | Open (before card sub-gate) |
| ADR-012 | Ten domains, one schema and producer role each, outbox between them (Baseline 2.0) | Accepted (Baseline 2.0 P1) |
| ADR-013 | PostgreSQL self-managed by Scanpay | Decided 21 Oct 2026 |
| ADR-014 | Approvals in a separate workflow domain with an execution contract | Proposed |
| ADR-015 | Cross-domain reads through owner-privilege contract views, forced row-level security | Proposed; depends on shared 0.7 (CR-7) |
| ADR-016 | Web framework for the business workspace and the operations console | Open |

Each proposed or open ADR gets an owner, alternatives, decision criteria, consequences and a decision deadline in Phase 0. Provider-integration and ledger ADRs are decided on verified capability, accounting and failure behaviour, not coding convenience.

**Cross-references:** requirements 01 §3–4 · flows 02 · client constraints 03 §5 · schema 04 · delivery and tests 06.
