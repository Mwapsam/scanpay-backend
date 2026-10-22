# Engineering Verification Standard

**Status:** Proposed  
**Applies to:** All Scanpay implementation, documentation, migration, security, testing, and release work.

## 1. Purpose

This document defines the minimum evidence required to claim that a change is correct, safe, and complete.

Verification is not a declaration of confidence. It is a record of what was checked, how it was checked, what passed, what failed, and what remains unknown.

The objective is to prevent unsupported claims, incomplete implementations, regressions, and unsafe assumptions from entering Scanpay.

## 2. Non-negotiable rules

1. **Inspect before changing.** Read the relevant source, specifications, existing implementation, tests, and repository instructions before proposing or making changes.
2. **Requirements before implementation.** Identify the applicable requirement IDs, user journey, data model, security controls, and acceptance criteria.
3. **Respect freeze gates.** Do not implement domain database changes or production migrations before the applicable chunk passes freeze gate 1.
4. **Evidence before claims.** Never claim a test passed, a requirement is satisfied, a migration is safe, or a feature is complete without supporting evidence.
5. **Test the failure paths.** Happy-path success is insufficient for financial operations, concurrency, authorization, retries, and external integrations.
6. **Use the real boundary when it matters.** Mocks may test isolated logic, but they cannot establish the behaviour of PostgreSQL transactions, roles, row-level security, or provider integration.
7. **Preserve uncertainty.** A timeout, missing callback, lost connection, or unknown provider response is not evidence of payment success, failure, or rollback.
8. **Do not invent missing decisions.** If correctness depends on an Open decision, identify the owner and stop or keep the affected behaviour safely blocked.
9. **Verify the final change.** Evidence gathered before the last relevant edit does not establish that the final version passes.
10. **Report limitations honestly.** A test not run is not a passing test. A mock is not a real integration. A local result is not staging verification.
11. **Keep specifications and implementation aligned.** Changes to contracts, states, schemas, permissions, or journeys require corresponding specification updates and review.
12. **Never bypass a gate to obtain a green result.** Do not disable, weaken, skip, or misrepresent a failing check to make a change appear verified.

## 3. Establish the verification plan

Before implementation, record or identify:

- The requirement ID or IDs in `01` and applicable acceptance criteria.
- The relevant journey and failure/recovery paths in `02`.
- The applicable UX rules and approved design in `03`.
- The relevant schema, ownership rules, and invariants in `04`.
- The applicable architecture, security, API, idempotency, and operational requirements in `05`.
- The phase, freeze gate, exit criteria, and test matrix in `06`.
- Relevant requirements in the Product and Financial Integrity Contract (`00`), Architecture Baseline, accepted ADRs, and approved domain drafts.
- Any Open decisions, unresolved conflicts, and required approvals.
- The tests and other evidence needed to establish correctness.

Use the actual repository paths and document versions. Do not assume filenames, commands, approved decisions, or implemented functionality without checking them.

If a required decision or freeze gate is missing, do not silently proceed as though it were approved.

## 4. Evidence levels

Use the following terms consistently.

| Level | Meaning |
|---|---|
| Inspected | Relevant code, specifications, or configuration have been read. No claim of correctness follows from inspection alone. |
| Implemented | A change exists in the code or documentation. Its correctness is not yet established. |
| Locally tested | Named checks were executed locally, with results recorded. |
| Integration verified | The relevant components and real boundaries were exercised together. |
| Independently reviewed | A reviewer examined the implementation and its evidence. State who or what performed the review. |
| Staging verified | The relevant behaviour was exercised in staging using the required environment, roles, integrations, and acceptance criteria. |
| Accepted | The authorized decision owner accepted the specified scope. Acceptance does not substitute for technical verification. |
| Verified | Required tests and evidence passed, applicable independent review is complete, and the relevant acceptance criteria are satisfied. |

These levels are not interchangeable. A change may be implemented but untested, tested but not independently reviewed, or technically verified but not accepted.

Do not mark a requirement, chunk, gate, or release Verified merely because its implementation exists.

## 5. Verification by change type

### 5.1 Rust code and domain logic

For changes to Rust code:

- Run the repository's applicable formatting and lint checks.
- Compile the affected crates and relevant dependent targets.
- Run unit tests for the changed behaviour.
- Test boundary values, invalid inputs, error paths, and relevant state transitions.
- Verify that state machines use explicit states and exhaustive transitions where required.
- Check that domain logic remains separate from I/O and that crate dependencies follow the approved architecture.
- Confirm that validated types from `crates/types` are reused instead of introducing raw strings, floating-point money values, or duplicate value types.
- Check for unintended changes to public APIs, event contracts, error semantics, and serialization.
- Run broader workspace checks when shared code, dependency declarations, root configuration, or cross-domain contracts change.

A successful compile establishes neither business correctness nor compliance with the requirements.

### 5.2 Database, SQL, and migrations

Database changes require evidence appropriate to their risk.

Before any production migration or domain table creation:

- Verify that the relevant domain chunk has passed freeze gate 1.
- Verify the approved schema design, ownership boundaries, and applicable decisions.
- Confirm that the change follows the domain's table ownership and producer-role rules.
- Check constraints, indexes, foreign-key policy, transaction boundaries, and rollback or recovery implications.
- Test the migration from the actual supported starting schema.
- Verify the resulting schema and constraints, rather than relying only on successful migration execution.
- Test upgrade and deployment behaviour relevant to the change.
- Confirm that migrations do not silently destroy, rewrite, or reinterpret existing business records.

Where database changes are prohibited by the current freeze status, use only an explicitly permitted disposable spike. Do not retain spike schema or code as production implementation.

#### Required real-PostgreSQL testing

Tests involving transactions, concurrency, database roles, or tenant isolation must run against real PostgreSQL with the relevant real roles, not only mocks.

Verify, as applicable:

- Concurrent operations and transaction isolation.
- Unique constraints and concurrent idempotency claims.
- Commit, rollback, and connection-loss behaviour.
- Database-enforced state-transition restrictions.
- Row-level security and forced row-level security.
- The actual privileges of each relevant role.
- Transaction-local organization context.
- Attempts to read or write another organization's data.
- Migration behaviour under the roles used in deployment.

A local PostgreSQL superuser bypasses ordinary row-level security protections. Results obtained under that role do not establish tenant isolation. Report this limitation explicitly.

### 5.3 Payments, ledger, and financial workflows

Financial verification must demonstrate the complete relevant lifecycle, not merely a successful API response.

Tests must be derived from the authoritative financial contract, applicable requirement IDs, provider failure matrix, and approved domain specifications.

Verify the applicable cases below.

**Payment lifecycle**

- Initiation and persisted idempotency identity.
- Pending and uncertain outcomes.
- Provider-confirmed success and provider-confirmed failure.
- Delayed, duplicated, and out-of-order callbacks.
- Provider queries and reconciliation.
- Retry after timeout, connection loss, or worker restart.
- Duplicate client requests and concurrent initiation attempts.
- Safe recovery after partial failure.
- Approved operator resolution, where implemented and authorized.

**Financial integrity**

- Only verified amounts enter the financial posting process.
- Each qualifying source event is posted at most once.
- Ledger corrections use explicit reversals or adjustments rather than editing immutable postings.
- Issued invoices are not silently edited.
- Approval authorizes the specified action but does not itself establish settlement.
- Refund and reversal behaviour follows the approved vocabulary and state machine.
- Financial records remain auditable and attributable to their source events.

**Mandatory unknown-outcome rule**

A timeout, missing callback, lost connection, or unknown provider answer must never be treated as proof of payment success or failure.

A retry must follow the approved idempotency and inquiry rules. It must not blindly resubmit a payment.

A test that mocks a provider response may establish local handling of that response. It does not establish the real provider's behaviour or prove end-to-end payment correctness.

**Financial acceptance evidence**

For a financial workflow, record the expected and observed state transitions, relevant idempotency behaviour, provider evidence, resulting financial records, and reconciliation outcome. Verify the actual database state as well as the returned API response.

### 5.4 APIs, events, workers, and external providers

Verify, as applicable:

- Request validation and authentication.
- Server-side authorization for each command.
- Idempotent handling of duplicate requests.
- Stable API and event contracts.
- Transactional outbox creation alongside the associated state change.
- At-least-once event delivery and consumer deduplication.
- Aggregate-version ordering where required.
- Retry, timeout, backoff, and recovery behaviour.
- Worker restart and repeated delivery.
- Malformed, delayed, duplicated, and unexpected external responses.
- Secret handling and safe error reporting.
- Provider-specific behaviour against the documented provider contract.

Do not infer MTN Zambia behaviour from generic Mobile Money documentation. Provider assumptions must be supported by the confirmed contract, suitable fixtures, or actual sandbox evidence.

Clearly distinguish simulated-provider, fixture-based, sandbox, and production evidence.

### 5.5 Security, permissions, and tenant isolation

For changes affecting security boundaries, verify the relevant controls directly.

Check, as applicable:

- Authentication and session handling.
- Authorization for every relevant server-side command.
- Organization isolation