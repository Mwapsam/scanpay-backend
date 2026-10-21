# Scanpay 04 — Backend Schema

Oct 18, 2026 · @TCS

## 1. Conventions and domain boundaries

This is the proposed PostgreSQL data model for the new, greenfield Rust modular monolith; nothing is inherited from the old database (00 GD-3). It is a design to review, and the ledger part is frozen only after finance signs off the accounting model (00 §5).

**Governed by 00.** States, commit boundary (T1–T4) and invariants I1–I11 are defined in the 00 Product and Financial Integrity Contract; this schema is how they are enforced. Payment tables are provider-neutral (00 GD-1): MTN Mobile Money is the first adapter; Airtel Money, Zamtel Kwacha and cards add rows, not tables. Limits and eligibility for the BoZ sandbox pilot are configuration (00 GD-2).

*Business summary.* Money is recorded in a double-entry ledger that cannot be edited, only added to. Every payment, payout and reversal can be traced from the customer's tap to the ledger line and the MTN reference.

**Version 1.2 (21 Oct 2026).** The data model is now organized by the ten domains of [Architecture Baseline 2.0](https://claude.ai/artifact/3TaqDBgNAXW9Y8P6cUW3xS). Sections 2–6 keep the payment, ledger and integrity design from 1.1, which 00 governs, but their module names predate the domains: the mapping below governs wherever they differ. Section 7 indexes the detailed domain drafts; section 8 specifies the seven domains not yet drafted, at design level. Nothing here is frozen or approved for migration; each chunk passes its own freeze gate (06 §2A). PostgreSQL is self-managed by Scanpay. 21 Oct 2026: consistency fixes (cross-schema FKs removed from §3 SQL, tenant column, overdue derived, messaging gate).

### Module-to-domain mapping (governs)

| 1.1 module or table | Baseline 2.0 domain and record | Change |
| --- | --- | --- |
| `identity.users`, credentials, devices, sessions, otp\_challenges | `shared` (0.7) | Owned by shared 0.7; not redefined here |
| `identity.businesses` | `shared.organizations` (0.7) | Column `business_id` everywhere becomes `organization_id` |
| `identity.memberships` (roles owner, manager, finance, cashier) | `organization.memberships` | Four roles: Owner, Finance, Cashier, Viewer (DEC-31 Open); no "manager". Organization 0.4 §3 |
| `identity.payout_destinations` | `organization.settlement_accounts` | Provisional on DEC-18; never edited in place (Organization 0.4 §4.6) |
| `identity.merchant_qr_codes` | `payments.qr_codes` | Moved to payments (§8.1) |
| `kyc.*` | `organization.kyb_*` | States: draft, submitted, in\_review, more\_info\_needed, approved, rejected, withdrawn; KYB approval never enables collection (Organization 0.4 §4.2) |
| `ledger.*` | `finance` | Same tables and invariants; schema renamed (§8.3) |
| `contacts.*`, `invoicing.*` | `commerce` | Issued invoices never edited: revisions after issue become approved adjustments (§8.2) |
| `notify.*` | `notifications` | §8.6 |
| `messaging.*` | `messaging` | Adds record cards, assignment and status posts (§8.5) |
| `ops.approvals` | `workflow` | Workflow Draft 0.3 |
| `ops.cases` | `support` (cases, disputes); `payments.escalations` (unresolved payments) | §8.1, §8.6 |
| `ops.reconciliation_*` | `finance` (statements, matches, reconciliation cases) | §8.3 |
| `platform.outbox_events`, `audit_log`, jobs | `shared` (0.7) | Contract requirements CR-1, CR-8, CR-9 |
| `platform.idempotency_keys` | Per-domain `command_keys` | Organization 0.4 §3.8 is the reference design |
| `platform.limit_policies`, `pilot_eligibility` | `organization.limit_definitions`, `limit_requirements`, `eligibility_decisions`; `payments.limit_windows`, `limit_reservations` | Policy in organization, atomic enforcement in payments (§8.1) |
| `platform.regulatory_reports` | Reporting read models | Not a domain; never writes business records |

### Conventions

- One PostgreSQL database, one schema per Baseline 2.0 domain (shared, organization, workflow, messaging, commerce, payments, retail, finance, notifications, support), each with its own producer role that writes only that schema.
- Primary keys: UUIDv7 (time-ordered) generated in Rust.
- Money: `amount_minor BIGINT` (ngwee) + `currency CHAR(3)`; never floating point. ZMW only at launch.
- Timestamps: `TIMESTAMPTZ`, stored UTC, displayed Africa/Lusaka.
- States: PostgreSQL enums or `TEXT` with `CHECK` constraints mirrored by Rust enums; transitions enforced in the domain layer and guarded in SQL (`UPDATE … WHERE state = $expected`).
- Every mutable table has `created_at`, `updated_at`, and `version INT` for optimistic concurrency where needed. Tenant tables carry organization\_id; row-level security keyed to `SET LOCAL app.organization_id` is forced on every base table (I9; ADR-015, Proposed). There are no cross-schema foreign keys (Baseline 2.0 §5.3): references to another domain are stored as refs and checked at command time through that domain's contract views.
- Financial and audit tables are append-only: no `UPDATE` or `DELETE` grants for the application role.
- PII columns flagged in a data catalogue with retention class (05 §4).

### Domain ownership

| Module | Owns | Reads from (via module API, not direct joins) |
| --- | --- | --- |
| identity | users, devices, sessions, businesses, memberships, roles | — |
| kyc | KYC applications, documents, decisions | identity |
| payments | payment intents, provider attempts, callbacks, payouts, refunds | identity, kyc, ledger |
| ledger | accounts, journal entries, postings, balances | — |
| notify | notifications, deliveries, preferences | identity |
| contacts | contacts, discovery settings | identity |
| invoicing | invoices, lines, revisions, allocations | identity, payments |
| messaging | conversations, members, messages, media, calls | identity |
| ops | cases, approvals, reconciliation runs and breaks | all (read) |
| platform | outbox, idempotency keys, audit log, jobs | — |

Superseded by the mapping above and Baseline 2.0 §5: domains interact only through commands, outbox events and read-only contract views; no cross-schema foreign keys; no domain writes another's tables.

## 2. Identity, businesses and KYC

Users are people; businesses are tenants that users join through memberships with roles, so every business-scoped row carries `business_id` for isolation.

| Table | Key columns | Constraints and notes |
| --- | --- | --- |
| `identity.users` | id, phone\_e164, display\_name, status (`active`, `locked`, `closed`), created\_at | `UNIQUE(phone_e164)`; phone is PII |
| `identity.credentials` | user\_id, kind (`passcode`, `device_key, passkey), secret_hash or public_key`, updated\_at | Argon2id hashes or public keys only; biometric data is never received or stored — biometrics only unlock a device-held key |
| `identity.devices` | id, user\_id, platform, push\_token, trusted\_at, revoked\_at | One active trust per device |
| `identity.sessions` | id, user\_id, device\_id, refresh\_token\_hash, expires\_at, revoked\_at | Index `(user_id) WHERE revoked_at IS NULL` |
| `identity.otp_challenges` | id, phone\_e164, purpose, code\_hash, attempts, expires\_at | Short TTL; attempt cap |
| `identity.businesses` | id, legal\_name, trading\_name, scanpay\_handle, status (`draft`, `active`, `suspended`, `closed`), kyc\_tier, created\_by | `UNIQUE(scanpay_handle)`; `active` requires approved KYC |
| `identity.memberships` | business\_id, user\_id, role (`owner`, `manager`, `finance`, `cashier`), status | `PRIMARY KEY(business_id, user_id)`; at least one owner |
| `identity.merchant_qr_codes` | id, business\_id, payload\_ref, status, issued\_at, revoked\_at | QR encodes a reference, never an account number |
| `identity.payout_destinations` | id, business\_id, provider, msisdn\_e164, verified\_at, status | Changes trigger KYC re-review (02 §2) |
| `kyc.applications` | id, business\_id, state (`draft`, `submitted`, `under_review`, `more_info_required`, `approved`, `rejected`, `suspended`), tier\_requested, submitted\_at | One open application per business (partial unique index) |
| `kyc.documents` | id, application\_id, doc\_type, storage\_key, sha256, uploaded\_by, uploaded\_at | Files in encrypted object storage; DB holds reference + hash |
| `kyc.decisions` | id, application\_id, reviewer\_id, decision, reason, decided\_at | Append-only |
| `kyc.document_access_log` | document\_id, actor\_id, reason, at | Every view logged |

**Assumption:** the KYC document types and tier limits are configuration (`kyc.tier_policies`), not hard-coded, until confirmed against Zambian requirements and MTN terms.

## 3. Payments and ledger

The ledger is the single financial truth: balanced, append-only journal entries, written in the same transaction as the payment state change that causes them, with database constraints that make a double posting impossible.

### Payment tables

| Table | Purpose | Key constraints |
| --- | --- | --- |
| `payments.providers` | Registry: provider code (`mtn_momo`, `airtel_money`, `zamtel_kwacha`, `card_{partner}`), method, enabled environments, stop-rule settings | Enabled per environment only after its gate |
| `payments.payment_intents` | One customer payment to one business | `UNIQUE(payer_user_id, idempotency_key)`; state per 00 §2.1; `amount_minor > 0` |
| `payments.provider_attempts` | Each external interaction (request, query, disbursement, refund) | Separate columns for `request_reference` (ours), `provider_transaction_id`, `correlation_id`; `UNIQUE(provider, request_reference, kind)` |
| `payments.provider_callbacks` | Every inbound callback, raw, with authentication result | Stored before processing; dedupe by `(provider, payload_hash)` |
| `payments.provider_evidence` | Verified outcomes (00 §2.3) | Immutable; `UNIQUE(provider, request_reference, outcome_source, source_ref)`; `no status column — processing decisions live in payments.evidence_dispositions` |
| `payments.state_transitions` | Append-only history for intents, payouts, refunds | Never updated |
| `payments.payouts` | Disbursement to a merchant payout destination | State per 00 §2.5; `UNIQUE(business_id, idempotency_key)` |
| `payments.refunds` | Money returned to the customer through a provider | State per 00 §2.6; own idempotency key and request reference; cumulative ≤ original (I8) |
| `ledger.reversals` | Internal corrections (00 §2.7) | Requires case ID and approval ID; approver ≠ proposer |
| payments.evidence\_dispositions | Append-only processing decisions per evidence row: verified, applied, superseded, conflicting, rejected | Actor, reason, time; latest row is current status; evidence row itself never changes (00 §2.3) |
| payments.provider\_capabilities | Versioned capability declaration per provider (00 §4 matrix): idempotent re-submission, not-found meaning, finality, stop rules | Read by the adapter at runtime; changes reviewed and audit-logged |

```sql
CREATE TYPE payments.intent_state AS ENUM (
  'created','submitting','pending_customer','uncertain',
  'under_review','succeeded','failed','rejected');

CREATE TABLE payments.payment_intents (
  id                     UUID PRIMARY KEY,
  payer_user_id          UUID NOT NULL,      -- ref to shared users; checked via contract view, no cross-schema FK
  business_id            UUID NOT NULL,      -- ref to the organization; checked via contract view, no cross-schema FK
  amount_minor           BIGINT NOT NULL CHECK (amount_minor > 0),
  fee_minor              BIGINT NOT NULL DEFAULT 0 CHECK (fee_minor >= 0),
  currency               CHAR(3) NOT NULL DEFAULT 'ZMW',
  provider               TEXT NOT NULL REFERENCES payments.providers(code),
  request_reference      UUID NOT NULL,      -- persisted in T1, reused for every query
  state                  payments.intent_state NOT NULL DEFAULT 'created',
  idempotency_key        TEXT NOT NULL,
  request_fingerprint    BYTEA NOT NULL,     -- 00 §4 idempotency
  invoice_id             UUID NULL,
  refunded_minor         BIGINT NOT NULL DEFAULT 0,
  refund_reserved_minor  BIGINT NOT NULL DEFAULT 0,
  next_check_at          TIMESTAMPTZ NULL,
  check_attempts         INT NOT NULL DEFAULT 0,
  created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (payer_user_id, idempotency_key),
  UNIQUE (provider, request_reference),
  CHECK (refunded_minor >= 0 AND refund_reserved_minor >= 0
         AND refunded_minor + refund_reserved_minor <= amount_minor)  -- I8 backstop
);
-- state, refunded_minor and refund_reserved_minor change only through
-- transition and refund functions that lock this row (00 §5 privilege model).
CREATE INDEX ON payments.payment_intents (next_check_at)
  WHERE state IN ('submitting','pending_customer','uncertain');
CREATE INDEX ON payments.payment_intents (business_id, created_at DESC);
```

There is no `reversed` flag: refunds and reversals are their own records, and an intent's net position is derived from them (00 R5).

### Ledger tables

| Table | Purpose |
| --- | --- |
| `ledger.accounts` | id, code, kind (`asset`, `liability`, `revenue`, `expense`), owner\_type, owner\_id, currency, status |
| `ledger.journal_entries` | id, source\_type, source\_id, purpose, effective\_at, created\_by — one business event |
| `ledger.postings` | id, journal\_entry\_id, account\_id, amount\_minor (signed: + debit, − credit), currency |
| `ledger.account_balances` | account\_id, balance\_minor, last\_posting\_id, version — maintained in the same transaction |

```sql
CREATE TABLE ledger.journal_entries (
  id           UUID PRIMARY KEY,
  source_type  TEXT NOT NULL,   -- 'payment_intent','payout','refund','reversal','reconciliation'
  source_id    UUID NOT NULL,
  effect       TEXT NOT NULL,   -- 'collection_success','payout_reserve','refund_complete',...
  fingerprint  BYTEA NOT NULL,  -- hash of lines, accounts, currency, evidence
  evidence_id  UUID NULL,       -- ref to payments evidence; no cross-schema FK
  approval_id  UUID NULL,       -- ref to a workflow approval; no cross-schema FK
  effective_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_by   UUID NULL,
  UNIQUE (source_type, source_id, effect),                    -- I3
  CHECK (evidence_id IS NOT NULL OR approval_id IS NOT NULL)  -- I10
);

CREATE TABLE ledger.postings (
  id               UUID PRIMARY KEY,
  journal_entry_id UUID NOT NULL REFERENCES ledger.journal_entries(id),
  account_id       UUID NOT NULL REFERENCES ledger.accounts(id),
  amount_minor     BIGINT NOT NULL CHECK (amount_minor <> 0),  -- sign convention set by finance
  currency         CHAR(3) NOT NULL
);

-- I1: balancing enforced at commit
CREATE CONSTRAINT TRIGGER journal_balanced
  AFTER INSERT ON ledger.postings
  DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION ledger.assert_entry_balanced();

-- I2 / P0-2: tables owned by scanpay_ledger; scanpay_app has no direct
-- INSERT/UPDATE/DELETE. All writes go through:
--   ledger.post_entry(source_type, source_id, effect, evidence_id,
--                     approval_id, lines jsonb) RETURNS uuid
-- SECURITY DEFINER, fixed search_path. On a duplicate effect key it
-- compares fingerprints: identical -> return existing id (replay);
-- different -> RAISE integrity incident.
```

### Account map (proposed)

| Account | Kind | Meaning |
| --- | --- | --- |
| `provider_cash:{provider}:collection` | asset | Funds held in Scanpay's collection account at each provider (MTN first) |
| `provider_cash:{provider}:disbursement` | asset | Funds available for payouts and refunds at each provider |
| `merchant_payable:{business}` | liability | Owed to a merchant; cannot go negative (I6) |
| `merchant_payout_reserved:{business}` | liability | Reserved for an in-flight payout |
| `refund_reserved:{business}` | liability | Reserved for an in-flight refund |
| `fee_revenue` | revenue | Scanpay fees, if a fee model is adopted |
| `suspense` | liability | Unexplained differences; every balance here has an open case |

Account names, directions and settlement treatment are placeholders until finance approves the accounting model (00 §5). Customer stored-value wallets are **not** enabled: holding customer balances likely requires regulatory authorisation. The account model supports them (`customer_wallet:{user}`) if that is later confirmed.

### Example postings (illustrative amounts; fee model is an open decision)

| Event | Debit | Credit |
| --- | --- | --- |
| Collection succeeds (T4), K 100.00, fee K 1.00 | provider\_cash:mtn\_momo:collection 100.00 | merchant\_payable 99.00; fee\_revenue 1.00 |
| Payout reserved | merchant\_payable 99.00 | merchant\_payout\_reserved 99.00 |
| Payout verified paid | merchant\_payout\_reserved 99.00 | provider\_cash:mtn\_momo:disbursement 99.00 |
| Payout verified failed | merchant\_payout\_reserved 99.00 | merchant\_payable 99.00 |
| Refund approved, K 99.00 | merchant\_payable 99.00 | refund\_reserved 99.00 |
| Refund verified complete | refund\_reserved 99.00 | provider\_cash:mtn\_momo:disbursement 99.00 |
| Reversal of a wrong posting | mirror of the original entry | mirror of the original entry |

Who bears the fee on a refund is an open commercial decision; the table shows the merchant's net amount only.

### Ledger invariants (tested in 06 §5)

1. **I1 Balanced** — deferred constraint trigger above.
2. **I2 Immutable** — ledger, evidence, dispositions and audit writable only through owner-role functions; no UPDATE/DELETE for any service role.
3. **I3 Exactly once** — unique effect key plus fingerprint comparison in `ledger.post_entry`.
4. **I4 Matching amounts** — amounts read from the locked source row inside T4; nightly verification.
5. **I5 Projection = journal** — projections updated only by the posting function under row lock; nightly recompute, mismatch pages on-call.
6. **I6 No overdraft** — reservation functions lock the merchant projection and reject if available < amount; `CHECK (balance_minor >= 0)` as backstop.
7. **I7 Legal transitions** — transition functions only; guarded update plus history row in one transaction.
8. **I8 Refund cap** — refund-approval function locks the payment row; completed + reserved ≤ original; row CHECK as backstop.
9. **I9 Tenant isolation** — tenant set per transaction with `SET LOCAL app.organization_id` after authorization; row-level security forced on every base table reads it; cross-tenant suite as proof (Organization 0.4 §7.2).
10. **I10 Evidence before money** — evidence or approval FK on every journal entry.
11. **I11 Limits** — limits defined by organization and enforced by payments, which reserves capacity under lock in the transaction that creates the intent; collection is refused when no limit policy is defined (§8.1).

Cross-table rules (I4, I6, I8) cannot be guaranteed by a row CHECK alone; the named functions are the enforcement and the CHECKs are backstops. Tests for each are in 00 §5.

The test for each is listed in 00 §5.

## 4. Notifications, contacts, invoicing and messaging

These domains reference payments and identities but never write financial tables; invoices learn about payments only through `payments` events.

### Notifications (G1)

| Table | Key columns | Notes |
| --- | --- | --- |
| `notify.notifications` | id, user\_id, kind, payload (non-sensitive), source\_event\_id, created\_at | `UNIQUE(user_id, source_event_id, kind)` prevents duplicate sends |
| `notify.deliveries` | id, notification\_id, channel (`push`, `in_app`, `sms`), state (`queued`, `sent`, `delivered`, `failed`), attempts, last\_error | Retry with backoff |
| `notify.preferences` | user\_id, kind, channel, enabled | Critical security and payment kinds cannot be disabled |

### Contacts (G2)

| Table | Key columns | Notes |
| --- | --- | --- |
| `contacts.contacts` | owner\_user\_id, target\_type (`user`, `business`), target\_id, nickname, created\_at | `PRIMARY KEY(owner_user_id, target_type, target_id)` |
| `contacts.discovery_settings` | user\_id, discoverable\_by\_phone, updated\_at | Default off for customers |

### Invoicing (G3)

| Table | Key columns | Notes |
| --- | --- | --- |
| `invoicing.invoices` | id, business\_id, customer\_user\_id NULL, customer\_name, number, state (`draft`, `issued`, `partially_paid`, `paid`, `cancelled`; overdue is derived, §8.2), currency, total\_minor, paid\_minor, due\_date, current\_revision | `UNIQUE(business_id, number)`; `paid_minor ≤ total_minor` |
| `invoicing.invoice_revisions` | invoice\_id, revision, lines snapshot (JSONB), totals, created\_by, created\_at | Append-only; issued revisions immutable |
| `invoicing.invoice_lines` | invoice\_id, revision, line\_no, description, quantity, unit\_price\_minor, tax\_code, tax\_minor, line\_total\_minor | Belongs to a revision |
| `invoicing.payment_allocations` | invoice\_id, payment\_intent\_id, amount\_minor | `UNIQUE(payment_intent_id, invoice_id)`; created only from a `succeeded` event |
| `invoicing.zra_submissions` | invoice\_id, revision, state, zra\_reference, response | **Reserved; unused** until the ZRA workstream is approved |

### Messaging and calls (G1 text and cards, G4 media)

| Table | Key columns | Notes |
| --- | --- | --- |
| `messaging.conversations` | id, kind (`direct`, `business`, `group`), created\_by, created\_at |  |
| `messaging.members` | conversation\_id, user\_id, role, joined\_at, left\_at, muted | Membership checked on every read |
| `messaging.messages` | id, conversation\_id, sender\_id, client\_message\_id, kind (`text`, `voice_note`, `payment_card`, `invoice_card`, `system`), body, media\_id, created\_at, deleted\_at | `UNIQUE(sender_id, client_message_id)`; partition by month when volume requires |
| `messaging.receipts` | message\_id, user\_id, delivered\_at, read\_at |  |
| `messaging.media` | id, owner\_id, storage\_key, mime, bytes, duration\_ms, sha256 | Object storage; retention policy open |
| `messaging.blocks` | blocker\_id, blocked\_id, created\_at |  |
| `messaging.reports` | id, reporter\_id, target\_type, target\_id, reason, state | Feeds ops |
| `messaging.calls` | id, conversation\_id, kind (`audio`, `video`, `conference`), state, started\_by, started\_at, ended\_at, end\_reason | Signaling state only; media never stored by default |
| `messaging.call_participants` | call\_id, user\_id, state, joined\_at, left\_at |  |

## 5. Platform: outbox, idempotency, audit and operations

Cross-cutting tables make retries safe and every action traceable.

| Table | Purpose | Key constraints |
| --- | --- | --- |
| `platform.idempotency_keys` | Stores request fingerprint and response for each client key | `PRIMARY KEY(scope, actor_id, key)`; scope = operation kind + tenant (00 §4 namespaces); request hash must match on replay; TTL cleanup |
| `platform.outbox_events` | Events written in the business transaction, published by a worker | `id` ordered; `published_at` NULL until sent; consumers dedupe by event id |
| `platform.jobs` | Durable background jobs (enquiry, payouts, reconciliation) | `SELECT … FOR UPDATE SKIP LOCKED`; `run_at`, attempts, max\_attempts, lease\_until and lease\_version fencing token checked on every write (expired leases reclaimed after a crash; stale workers' writes rejected), dead\_lettered\_at for poison jobs (alerted, never silently dropped) |
| `platform.audit_log` | Every privileged or sensitive action | Append-only; actor, action, target, reason, request id, at |
| `ops.cases` | Investigations and disputes | Links to intent/payout; state, assignee, resolution |
| `ops.approvals` | Second-person approval for money-moving actions | Approver ≠ requester (CHECK) |
| `ops.reconciliation_runs` | Each reconciliation execution | Period, source file hash, counts |
| `ops.reconciliation_breaks` | Mismatches found | Type, references, state, linked case |
| platform.limit\_policies | Configurable limits: per transaction, per day, per user, per merchant, per provider, per sandbox cohort | Versioned; changes audit-logged and dual-approved (I11) |
| platform.pilot\_eligibility | Users and businesses admitted to the sandbox pilot, with consent and disclosure records | Consent timestamp and disclosure version required |
| platform.regulatory\_reports | Generated reports required by BoZ, once specified | Immutable once submitted; source query and data hash stored |

### Index and partitioning plan

- Hot lookups: intent by id, by `(payer_user_id, created_at DESC)`, by `(business_id, created_at DESC)`, by provider reference.
- Worker scans use partial indexes on non-terminal states only.
- Postings indexed by `(account_id, id)` for statements.
- Partition `ledger.postings`, `messaging.messages` and `platform.audit_log` by month only when measured volume justifies it (06 §5 load tests).

## 6. Clean start, open questions and cross-references

The schema is created empty by forward-only migrations; no legacy schema, data or balances are imported (00 GD-3, §6).

### Setup

1. Roles (`scanpay_owner`, `scanpay_ledger`, `scanpay_app`, `scanpay_worker`, `scanpay_ops_read`) and grants are created by the first migration and tested in CI.
2. Ledger tables, posting and transition functions are created only after finance approves the account model.
3. Every migration is reviewed, forward-only and run against a real PostgreSQL instance in CI.
4. Seed data is limited to configuration (providers, limit policies, account templates); no customer or financial data is seeded in production.

### Open questions

- [ ] Finance approval of account meanings, sign convention and settlement treatment (00 §5).
- [ ] Settlement model (instant vs batch payouts) changes payout volume and account design.
- [ ] Fee model and who bears MTN charges.
- [ ] Retention periods for KYC documents, messages and voice notes.
- [ ] Whether customer stored-value wallets are permitted.

**Cross-references:** requirements 01 §3 · state machines 02 §1 · status labels 03 §3 · data access, encryption and retention 05 §4 · clean start and invariant tests 06 §4–5.

## 7. Domain index

| Chunk | Domain | Specification | State |
| --- | --- | --- | --- |
| 1 | shared | Shared draft 0.7 (owned separately; its text was not available to the architecture review) | Not frozen; CR-1 to CR-11 pending |
| 2 | organization | [Organization Draft 0.4](https://claude.ai/artifact/K1jPBwUKMcLFrJxCxbaoRF) | Proposed; reviewed twice |
| 3 | commerce | §8.2 | Design level, Draft 0.1 |
| 4 | workflow | [Workflow Draft 0.3](https://claude.ai/artifact/7Re3AyWYUhyf6Z4fjAbE72) | Proposed; reviewed twice |
| 5 | retail | §8.4 | Design level, Draft 0.1 |
| 6 | payments | §3 (intent, evidence, refund, payout tables) and §8.1 | Design level, Draft 0.1 |
| 7 | finance | §3 (ledger) and §8.3 | Design level, Draft 0.1 |
| 8 | messaging | §8.5 | Design level, Draft 0.1 |
| 9 | notifications, support | §8.6 | Design level, Draft 0.1 |

"Design level" means records, key fields, constraints, states, events and open decisions. Each design-level domain still needs a full draft in the form of Organization 0.4 (field types, command contracts, contract views, error classes, acceptance tests) before its freeze gate. Every domain follows the shared conventions set there: command keys with stored refusals (§3.8), one evaluation instant per command, forced row-level security, owner-privilege contract views, and start-dated series for time-based policy.

## 8. Remaining domains (design level, Draft 0.1)

### 8.1 payments (chunk 6)

Owns payment intents, attempts, evidence, escalations, refunds, provider reversals, receipts, QR codes, and limit reservations. Builds on the §3 tables; the changes and additions are below. State names follow 00 until N13 is decided.

| Table | Key fields | Constraints and rules |
| --- | --- | --- |
| `payment_intents` (extends §3) | adds `organization_id` (replaces business\_id), `origin_kind` (`invoice`, `pos_tender`, `standalone`), `invoice_ref`, `pos_sale_ref`, `tender_ref`, `origin_thread_ref`, `customer_ref`, `purpose`, `payer_msisdn`, `requested_by_membership_ref`, `verified_amount_minor`, `correlation_id`, `aggregate_version`, `gate_a_compliance_version`, `gate_b_compliance_version` | `payer_user_id` becomes nullable: a customer paying a request need not be a Scanpay user. CHECK exactly one origin: `invoice_ref`, or `pos_sale_ref` + `tender_ref`, or `standalone` with `customer_ref` and `purpose` (N10 Open). Partial UNIQUE (`invoice_ref`) WHERE state is open (`created`, `submitting`, `pending_customer`, `uncertain`, `under_review`). Partial UNIQUE (`tender_ref`). Receipts, allocations and postings use `verified_amount_minor` |
| `limit_windows` | `organization_id`, `limit_definition_ref`, `window_start`, `reserved_minor`, `consumed_minor`, `version` | Counter cache; locked FOR UPDATE in a fixed order when an intent is created; nightly recompute from reservations, drift alerts |
| `limit_reservations` | `intent_id`, `window` key, `amount_minor`, `limit_definition_ref`, `status` (`reserved`, `consumed`, `released`), `release_reason` | Source of truth for counters. Held while the intent is open; consumed at the verified amount on success; released on failure or rejection; restoration after refund or provider reversal is N4 Open |
| `refunds` (extends §3) | adds `approval_request_ref` (UNIQUE), `requested_by_membership_ref`, `authorized_at`, `executed_at` | States per 00 §2.6, with `authorized` recorded before any provider call (emits `refund.authorized`, Workflow W2). Refundable remainder checked under a lock on the intent; in-flight refunds count against it |
| `provider_reversals` | `intent_id` (UNIQUE), `provider_report_ref`, `amount_minor`, `reported_at` | Created only from a provider report; full amount; never from a staff action |
| `escalations` | `intent_id` or `refund_id`, `opened_reason`, `state` (`open`, `resolution_proposed`, `closed`), `proposed_outcome`, `evidence_document_ref`, `approval_request_ref` | Operator resolution needs written provider evidence and a second staff member (C2 Open). Closing emits the payment's normal final event |
| `receipts` | `organization_id`, `number`, `intent_id` or `refund_id`, `amount_minor`, `issued_at`, `reversed_by_ref` | Created only on a verified outcome. Numbers gapless per organization, assigned under the organization's payments lock. A reversal adds a linked record; the receipt is never edited |
| `qr_codes` | `organization_id`, `payload_ref` (high-entropy), `status`, `issued_at`, `revoked_at` | Resolving a QR runs the same eligibility gate as any request |
| `command_keys` | As Organization 0.4 §3.8 | Client keys for customer payments, member requests and refunds |

**Gates.** Gate A (intent creation) and gate B (before each provider call) read organization's `collection_policy_v1` and `effective_limits_v1` at the command's evaluation instant and fail closed. A request refused at gate B before any provider call ends `rejected` with reason `collections_blocked` (Baseline calls this `voided`; N13).

**Events:** `payment.verified`, `payment.failed`, `payment.rejected` (or `voided`, N13), `payment.reversed`, `refund.authorized`, `refund.submitted`, `refund.verified`, `refund.failed`, `escalation.opened`, `escalation.closed`. None for in-between states. **Consumes:** `eligibility.recorded`, `restriction.*`, `limit.recorded` (cache invalidation only), `approval.decided`, `approval.expired`. **Contract views:** `payment_status_v1` (for messaging cards and retail tenders: state, verified amount, version), `refundable_amount_v1`.

**Open:** N13, C1, C2, C5, C6, DEC-27, N4, N8, N10, N11, N12; MTN refund API support. **Tests:** Baseline PAY-01–PAY-30, T01–T15 (06 §5).

### 8.2 commerce (chunk 3)

Owns customers and contacts, products and prices, invoices, adjustments and allocations. Replaces 1.1's `contacts` and `invoicing` modules.

| Table | Key fields | Constraints and rules |
| --- | --- | --- |
| `customers` | `organization_id`, `display_name`, `phone_e164`, `user_ref` (nullable), `origin_thread_ref` | UNIQUE (`organization_id`, `phone_e164`) where phone present; phone is PII |
| `personal_contacts` | `owner_user_ref`, `target_kind` (`user`, `organization`), `target_ref`, `nickname` | PK (`owner_user_ref`, `target_kind`, `target_ref`); discovery is opt-in (CON-04) |
| `products`, `prices` | `organization_id`, name, unit, `price_minor`, currency, effective range | Prices are a start-dated series; an invoice line copies the price at issue |
| `invoices` | `organization_id`, `customer_id`, `number` (assigned at issue), `state` (`draft`, `issued`, `partially_paid`, `paid`, `cancelled`), currency, `total_minor`, `due_date`, `origin_thread_ref`, `version` | UNIQUE (`organization_id`, `number`). Lines and totals editable only in `draft`. Overdue is derived (due date passed, balance > 0), not stored. Cancel needs an approved request and zero net allocations |
| `invoice_lines` | `invoice_id`, `line_no`, description, quantity, `unit_price_minor`, `tax_code`, `tax_minor`, `line_total_minor` | Frozen when the invoice is issued (trigger) |
| `invoice_adjustments` | `invoice_id`, `amount_minor` (signed), reason, `approval_request_ref` (UNIQUE), `created_at` | Append-only; the only way an issued invoice's total changes (DEC-25 Open) |
| `allocations` | `invoice_id`, `source_event_type`, `source_ref`, `amount_minor` (signed), `created_at` | Append-only; UNIQUE (`source_event_type`, `source_ref`, `invoice_id`); created only from consumed payment, refund or reversal events. Balance = total + adjustments − allocations |
| `fiscal_submissions` | reserved | Unused until DEC-17 is decided |

**Rules.** A verified payment above the outstanding balance: allocation capped at the balance, excess handling N9 Open. A provider reversal unwinds the allocation as a new negative allocation (C4 proposed `cascade_unwind`); history is never deleted.

**Events:** `invoice.issued`, `invoice.adjusted`, `invoice.cancelled`, `allocation.recorded`. **Consumes:** `payment.verified`, `payment.reversed`, `refund.verified`, `approval.decided`, `approval.expired`. **Contract view:** `invoice_balance_v1` (for payments' one-open-request check and messaging cards).

**Open:** DEC-25, DEC-26, DEC-17, C4, N9. **Tests:** Baseline COM-01–COM-04; to be expanded in the full draft.

### 8.3 finance (chunk 7)

Owns the ledger (§3 tables, renamed to schema `finance`), statement imports, matching and reconciliation. Account names wait for DEC-01 and DEC-18.

| Table | Key fields | Constraints and rules |
| --- | --- | --- |
| `accounts`, `journal_entries`, `postings`, `account_balances` | As §3 | I1–I11 unchanged. `journal_entries` add `source_event_type`, `source_ref`, `posting_kind`; UNIQUE on the three, so a redelivered event never posts twice |
| `statement_imports` | `provider`, `period`, `file_document_ref`, `file_sha256`, `imported_by`, `imported_at` | UNIQUE (`provider`, `file_sha256`); the file itself is a shared document |
| `statement_lines` | `import_id`, `line_no`, `provider_reference`, `amount_minor`, `direction`, `value_date`, `raw` | Append-only; never edited |
| `matches` | `statement_line_id` (UNIQUE), `target_kind` (`payment`, `refund`, `payout`), `target_ref`, `match_rule`, `matched_at` | One match per line; unmatched lines open a case |
| `recon_cases` | `organization_id` (nullable for platform-level), `kind` (`unmatched_line`, `amount_mismatch`, `status_mismatch`, `missing_line`, `duplicate_reference`), `state` (`open`, `decision_proposed`, `approved`, `closed`, `escalated`), refs, `version` | A case never changes a payment's state; a status mismatch is attached as evidence to the payment's escalation (C3 proposed) |
| `recon_decisions` | `case_id`, `proposed_by`, `decision`, `adjusting_journal_ref`, `approval_request_ref` (UNIQUE) | Approver is a Finance member who did not propose it (N3 Open for Owner fallback) |

**Posting sources:** `payment.verified` (collection), `refund.verified` (refund), `payment.reversed` (reversing journal; parks until the original posting exists), `pos_sale.completed` for cash tenders, `till.closed` for cash differences, approved reconciliation decisions (adjusting journal). Approval events are never a posting source (R4).

**Events:** `posting.created`, `recon.case.opened`, `recon.case.decision_approved`, `recon.case.closed`. **Open:** DEC-01, DEC-18, DEC-17, C3, N3. **Tests:** Baseline FIN-01–FIN-06; ledger invariant suites (06 §5).

### 8.4 retail (chunk 5)

| Table | Key fields | Constraints and rules |
| --- | --- | --- |
| `tills` | `organization_id`, `branch_ref` (nullable while branches are out of scope, U6), name, status | — |
| `till_sessions` | `till_id`, `opened_by_membership_ref`, `float_minor`, `state` (`open`, `counting`, `closed`), `expected_cash_minor`, `counted_cash_minor`, `difference_minor`, `closed_at` | One open session per till (partial UNIQUE). Cannot close while any sale in it is `held` (trigger). Float and acceptable-difference rules DEC-37 Open |
| `pos_sales` | `till_session_id`, `customer_ref` (nullable), `state` (`cart`, `tendering`, `completed`, `held`, `abandoned`), `total_minor`, `correlation_id` | Cannot move to `abandoned` while an electronic tender is `pending` (trigger) |
| `sale_lines` | `sale_id`, `product_ref`, quantity, `unit_price_minor`, `line_total_minor` | Frozen once tendering starts |
| `tenders` | `sale_id`, `kind` (`cash`, `electronic`), `amount_minor`, `state` (`pending`, `confirmed`, `failed`, `cancelled`), `payment_intent_ref` | Partial UNIQUE (`sale_id`) WHERE kind = electronic AND state <> cancelled. A cash tender cannot be added while an electronic tender on the sale is pending (DEC-37 proposed). An electronic tender is confirmed only by `payment.verified` |
| `returns` | `sale_id`, lines, `amount_minor`, `approval_request_ref`, `refund_ref`, `state` | A refund goes through workflow and payments; cash refunds DEC-37 Open |

**Events:** `pos_sale.completed`, `till.closed`, `return.requested`. **Consumes:** `payment.*`, `refund.*`, `approval.decided`. **Open:** DEC-37, U6. **Tests:** Baseline RET-01–RET-03.

### 8.5 messaging (chunk 8)

| Table | Key fields | Constraints and rules |
| --- | --- | --- |
| `threads` | `kind` (`business_customer`, `direct`, `group`), `organization_id` (for business threads), `customer_ref`, `created_at` | One business thread per (organization, customer) unless split is decided later |
| `participants` | `thread_id`, `user_ref`, `membership_ref`, `role`, `joined_at`, `left_at` | Every read checks participation (or business membership with `chat.read`) |
| `assignments` | `thread_id`, `assignee_membership_ref`, `assigned_by`, `assigned_at` | Append-only; the latest row is the current assignee |
| `messages` | `thread_id`, `sender_user_ref`, `sender_membership_ref`, `acting_as` (`self`, `business`; DEC-32 Open), `client_message_id`, `kind` (`text`, `record_card`, `system`, later `voice_note`), body, `created_at`, `deleted_at` | UNIQUE (`sender_user_ref`, `client_message_id`) |
| `record_cards` | `thread_id`, `source_domain`, `source_type`, `source_id`, `display_state`, `amount_minor`, `last_source_version`, `updated_at` | UNIQUE (`thread_id`, `source_type`, `source_id`). Projection: applies an event only if its version is newer; refreshes in-between payment states from `payments.payment_status_v1`; never shows paid before `payment.verified` |
| `status_posts` | `author_kind` (`user`, `organization`), author ref, `posted_by_membership_ref`, body, `media_ref`, `expires_at` (chosen by the author), `state` (`draft`, `live`, `ended`, `removed`, `taken_down`) | Posting for a business needs `status.post`. Expiry options, ranking and boosting DEC-36 Open |
| `status_views`, `status_reports` | `post_id`, viewer or reporter, reason, `created_at` | A report emits `status.reported` for moderation in support |
| `blocks` | `blocker_ref`, `blocked_ref` | As 1.1 |

Calls, conferences and voice notes keep the 1.1 tables (§4) and are designed fully before G4.

**Events:** `message.sent`, `status.posted`, `status.reported`. **Consumes:** `invoice.*`, `payment.*`, `refund.*`, `membership.*`. **Open:** DEC-32, DEC-36. **Tests:** Baseline MSG-01; to be expanded.

### 8.6 notifications and support (chunk 9)

**notifications** keeps 1.1's notifications, deliveries and preferences (§4), plus:

| Table | Key fields | Constraints and rules |
| --- | --- | --- |
| `templates` | `kind`, `channel`, `locale`, `version`, body | Versioned; lock-screen variants carry no amounts or names (NOT-04) |
| `notifications` | adds `source_event_id`, `recipient_ref`, `kind` | UNIQUE (`source_event_id`, `recipient_ref`, `kind`): a replayed event never sends twice. Never says paid without `payment.verified` |
| `send_requests` | `idempotency_key`, `channel`, `recipient`, `requested_by_domain`, `state` | Direct send interface used by organization's invitation delivery job; UNIQUE (`requested_by_domain`, `idempotency_key`) |

**support** owns cases, disputes and moderation:

| Table | Key fields | Constraints and rules |
| --- | --- | --- |
| `cases` | `organization_id` (nullable), `kind` (`support`, `dispute`, `approval_conflict`, `recon_escalation`, `moderation`), `state` (`open`, `in_progress`, `waiting`, `resolved`, `closed`), `assignee_staff_ref`, `opened_from_event_id` | UNIQUE (`opened_from_event_id`) so an event opens one case. A case never changes a payment, refund, membership or policy record; corrections go through the owning domain's commands (Workflow W11) |
| `case_links` | `case_id`, `target_domain`, `target_type`, `target_ref` | Links to payments, refunds, approvals, posts |
| `case_events` | `case_id`, `actor`, `action`, `note`, `at` | Append-only |
| `moderation_items` | `post_ref`, `report_count`, `decision`, `decided_by` | A take-down is a messaging command, not a direct write |

**Events:** `notification.delivery_failed`, `case.opened`, `case.closed`. **Consumes:** most domain events (notifications); `status.reported`, `recon.case.*`, `escalation.*`, conflicted approvals (support). **Open:** DEC-36, SMS vendor.
