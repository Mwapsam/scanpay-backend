# Scanpay

Scanpay is a Zambia-first business operating system: customers and businesses chat, invoice, take payments and run their books in one place ("Pay. Connect. Grow."). Payments start with MTN Mobile Money; Airtel Money, Zamtel Kwacha and cards follow, each behind its own gate. This is a greenfield rebuild: no legacy code, schema or data is carried forward.

## Project status (read first)

- **Pre-development.** The specifications exist; the code does not. Nothing is built, tested or verified yet.
- **No database changes before freeze.** Do not write migrations or create tables for a domain until that domain's design chunk has passed its freeze gate (06 §2A). As of 21 Oct 2026, no chunk is frozen.
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

- Backend: Rust modular monolith (Tokio, Axum, SQLx proposed), API and worker processes.
- Database: PostgreSQL, self-managed by Scanpay, one schema per domain.
- Mobile and tablet: React Native with Expo. Web business workspace and operations console: frameworks undecided (ADR-016).
- Hosting: Infratel Zambia (confirmation pending).
- Only the backend talks to MTN, SMS and KYB providers; clients never hold provider credentials.

## Architecture rules

- **Ten domains:** shared, organization, workflow, messaging, commerce, payments, retail, finance, notifications, support. Reporting is read models, not a domain. Do not add domains such as communication, growth or services.
- **One writer:** each table belongs to one domain's schema and producer role. No domain writes another's tables. No cross-schema foreign keys; other domains are read only through named read-only contract views.
- **Events:** written to the outbox in the same transaction as the change; delivered at least once; consumers deduplicate by event id and order by aggregate version.
- **Approvals** live in `workflow`. An approval authorizes one exact command once; the owning domain rechecks and executes it. Approval never means money moved.
- **Tenant isolation:** `organization_id` on every business row; `SET LOCAL app.organization_id` after authorization; row-level security forced on every table; permission checked on the server for every command.

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

## Coding conventions

- Money: integer minor units (`BIGINT`) plus `CHAR(3)` currency. Never floating point.
- Time: `timestamptz` in UTC; display in Africa/Lusaka. In a command, capture one evaluation instant with `clock_timestamp()` after taking locks and use it for every time check. Never use `now()` in a check: it is fixed at transaction start.
- Commands: insert the command key first, then a savepoint. Business refusals roll back to the savepoint and are stored, so a retry returns the same refusal. Serialization failures, deadlocks, timeouts and connection loss roll back the whole transaction (Organization 0.4 §3.8).
- A lost connection is an unknown outcome. Check the command key; never assume rollback.
- State machines are Rust enums with exhaustive transitions, backed by database triggers that reject illegal transitions.
- Time-based policy (eligibility, restrictions, limits, settlement accounts) is a series of immutable start-dated rows; scheduled rows can be cancelled before they start; rows never close or restore one another.
- Tests that involve concurrency, roles or transactions run against real PostgreSQL with the real roles, not mocks.

## Open decisions: do not assume an answer

These are Open until a named owner signs them off (full register: Baseline 2.0 §15, 06 §6). Code and docs must keep them visible, not settle them by implementation.

- **N13** One vocabulary for payment states and for refund vs reversal. Until decided, use 00's state names (`uncertain`, `under_review`, `succeeded`…) in code and APIs; 02 has the mapping to UI labels.
- **N1** Who approves refunds in a business with a single Owner. Until decided, such requests cannot be created.
- **DEC-31** business and staff roles · **DEC-34** collecting during KYB review (off) · **DEC-35** limit values · **DEC-23** approval thresholds and expiry · **DEC-37** till and cash rules · **DEC-01 / DEC-18** chart of accounts and whether Scanpay holds funds · **C2 / C3** operator resolution, statement lines as evidence · **DEC-27** direct MTN vs aggregator · MTN refund API support.

## Working in this repo

- Before implementing a feature, find its requirement ID in 01, its journey in 02, its tables in 04 and its controls in 05, and check that its chunk is frozen (06 §2A).
- When something needs a decision that isn't recorded, stop and ask, or mark it Open with a recommended option. Don't invent product, finance, compliance or regulatory answers.
- Don't invent provider behaviour. MTN Zambia's contract is confirmed in writing, not assumed from generic MoMo documentation.
- Keep docs and code in step: if a change alters a contract, state machine or table, the matching document needs the same change.