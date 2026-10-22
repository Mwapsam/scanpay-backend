# Scanpay 06 — Implementation Plan

Oct 18, 2026 · @TCS

## 1. Approach

We deliver the full scope (01 §2) through six gates, starting with a discovery phase and a payment-integrity foundation; no dates are committed until discovery sizes the work.

**Governed by 00.** Exit criteria below prove the 00 Product and Financial Integrity Contract. Strategy: shared financial-integrity foundation (G0) → MTN end to end (G1) → Airtel Money, Zamtel Kwacha and cards as independently validated adapters (G1+) → pilot within the BoZ Regulatory Sandbox once accepted (G5). A provider in the architecture is not available to users until its sub-gate passes. This is a greenfield build (00 GD-3, GD-4): Rust backend, React Native (Expo) app and Infratel Zambia hosting. There is no data migration or legacy refactoring; the old system is retired under §4.

*Business summary.* The plan front-loads the riskiest, most valuable work — money that is always correct — and makes every later feature earn its place by passing tests and operational checks. Leadership approves each gate on evidence, not on a calendar.

**Version 1.2 (21 Oct 2026).** Adds the architecture design-and-freeze stage that now runs inside Phase 0 (§2A), updates G1–G3 to the scope in PRD 1.2, and records the 21 October decisions. Nothing in this plan authorizes production code or database changes before the relevant design chunk passes its freeze gate. 21 Oct 2026 (later): gate 1 asks for test cases specified, not test code; adds the optional pre-freeze spike and defines the MVP as the first build slice (§2A).

&#91;embedded content: delivery phases and gates · not to scale\]

The accent marks G0: nothing customer-facing ships until payment integrity is proven. G5 is a readiness check for the scope BoZ approves; it may follow G1 directly and need not wait for later gates.

### Working rules

- Each gate has written exit criteria (§5); the gate review is a meeting with evidence, signed off by product, engineering and operations.
- Features for later gates sit behind flags; code may merge early, but it is not enabled for users before its gate.
- The old system is retired under the controls in §4; its code and data are not carried forward.
- Estimates are produced at the end of Phase 0 and re-forecast at every gate.

## 2. Phase 0 (Discovery) and G0 (Foundation)

Discovery replaces assumptions with facts; G0 builds and proves the payment-integrity core before any user-facing rebuild work is enabled.

### Phase 0 — Discovery

| Workstream | Output | Owner (to assign) |
| --- | --- | --- |
| Legacy containment | Every legacy credential rotated or revoked; old provider-integration code read for lessons only; list of outstanding merchant balances and unresolved payments to settle | Engineering lead |
| Clean foundations | Repositories, environments, CI/CD, secrets management, Rust service skeleton, PostgreSQL with service roles (04 §6), Expo app skeleton | Engineering |
| Provider confirmation | MTN Zambia's current contract in writing (API version, callback authentication, status query, rate limits, statements, sandbox, terms); first assessment of Airtel, Zamtel and card partners; direct vs aggregator evaluation (ADR-010); capability matrix per provider (00 §4) | Product + partnerships |
| Regulatory check | Which obligations apply (05 §4); BoZ sandbox eligibility, application contents (test plan, safeguards, exit strategy) and expected conditions | Legal / compliance |
| Accounting model | Finance sign-off on accounts, sign convention, settlement model and fees (00 §5) | Finance + engineering lead |
| Infratel confirmation | Written confirmation of arrangement, data locations, security controls and DR site; proposed RPO/RTO. PostgreSQL itself is self-managed by Scanpay (decided 21 Oct), so backups, WAL archiving and restore drills are Scanpay's | Engineering lead |
| Estimates | Sized backlog for G0–G1; first forecast for G2–G4 | Engineering + product |
| Immediate risk fixes | Rotate exposed secrets in the current system now | Engineering |

**Exit:** all of the above documented and reviewed; open decisions in 01 §9 each with a named owner, a deadline or gate dependency, and a consequence (register in §6).

### 2A. Architecture design and freeze (within Phase 0)

The system is designed domain by domain in nine chunks ([Baseline 2.0](https://claude.ai/artifact/3TaqDBgNAXW9Y8P6cUW3xS) §16). Chunks can be drafted in parallel; each is frozen only when its decisions and its contracts with neighbouring chunks are settled.

| Chunk | Domain | Status on 21 Oct 2026 | Blocking decisions |
| --- | --- | --- | --- |
| 1 | shared | Draft 0.7 exists; not frozen; its text was not available to the architecture review | 0.7 freeze checklist; CR-1 to CR-11 |
| 2 | organization | [Draft 0.4](https://claude.ai/artifact/K1jPBwUKMcLFrJxCxbaoRF) | DEC-31, 34, 35, 23, 28–30; N1, N2, N5, N6, N7, N12; KYB evidence list |
| 3 | commerce | Specified in 04 §8 (design level) | DEC-25, 26, 17; C4; N9 |
| 4 | workflow | [Draft 0.3](https://claude.ai/artifact/7Re3AyWYUhyf6Z4fjAbE72) | DEC-23, 29/30, 31; N1, N3 |
| 5 | retail | Specified in 04 §8 (design level) | DEC-37 |
| 6 | payments | Specified in 04 §8 (design level) | C1, C2, C5, C6, N13; DEC-27, 34, 35; N4, N8, N10, N11 |
| 7 | finance | Specified in 04 §8 (design level) | DEC-01, 18, 17; C3 |
| 8 | messaging | Specified in 04 §8 (design level) | DEC-32, 36 |
| 9 | notifications, support | Specified in 04 §8 (design level) | DEC-36 |

Each chunk passes three gates in order (Baseline §16.2):

1. **Freeze readiness:** blocking decisions signed by their owners; shared 0.7 frozen; schema, transitions and events specified and consistent; contracts reviewed by both sides; security review; acceptance test cases specified: inputs, expected outcomes, and the invariant or requirement each proves, covering concurrency, idempotency, authorization, policy and failure recovery. Test code is written during the build. Passing it authorizes migrations (reviewed by a second engineer) and code for that chunk.
2. **Implementation acceptance:** the tests pass in CI and on staging against the real implementation.
3. **Verification:** someone other than the implementer reviews the evidence; only then is an item marked Verified.

**Pre-freeze spike (optional).** Before any chunk is frozen, throwaway code may run against a disposable database to answer questions only a build can answer: PostgreSQL behaviour (`transaction_timeout`, row-level security through contract views), whether SQLx fits, and how the MTN sandbox behaves (DEC-27). Its schema is not the production schema and none of its code is kept. Its findings are first recorded as observed in `docs/findings/`, then carried into the chunk draft, 05 or an ADR; only after that is the spike deleted. A spike is not a database change under the freeze rule.

**MVP: the first build slice.** A business requests payment from a conversation, through the MTN sandbox; pending and timeout handled correctly; outcome verified; receipt issued; ledger posted; duplicate and late answers shown harmless; unresolved payments never falsely settled.

- **Scope:** shared (chunk 1), organization (2), the minimum of workflow (4), payments for MTN collection (6) and finance posting (7); commerce (3) only if the slice uses invoices; and the messaging record-card contract. Only these chunks pass gate 1, and only for what the slice uses. Every other chunk stays at design level.
- **Proof:** the MVP proves G0 against the simulated provider, then G1 against the MTN sandbox. Its passing tests are the gate 2 evidence for those chunks; an independent review of that evidence is gate 3.
- **No real customer money before G5.** That needs BoZ sandbox approval and the two-approver initiation switch (00 §8, 05 §4). Before then the MVP runs only on the simulated provider, the MTN sandbox, or a closed internal trial if the BoZ conditions allow it.
- **Blockers to starting:** shared 0.7 frozen (CR-1 to CR-11 pending) and a full payments draft in the form of Organization 0.4.

An open decision blocks only the chunks listed against it; independent chunks keep moving.

### G0 — Foundation (payment integrity and security)

- [ ] Cargo workspace, CI (build, test, clippy, audit), reproducible container, staging environment
- [ ] PostgreSQL schemas for the frozen chunks (shared, organization, workflow, payments, finance at minimum), one schema and producer role per domain (04)
- [ ] Ledger module with invariants I1–I11 enforced and tested (04 §3)
- [ ] Per-domain command keys with stored refusals and error classes (05 §2, Organization 0.4 §3.8)
- [ ] Transactional outbox and Postgres job queue
- [ ] Provider abstraction + simulated provider scripting F1–F15 (00 §4); MTN sandbox connectivity spike only — real-provider proof belongs to G1
- [ ] Payment state machine with guarded transitions, enquiry and escalation jobs
- [ ] Authentication, sessions, authorization policies, rate limiting (05 §4)
- [ ] Secret store, audit log, observability baseline, ledger-invariant alerts
- [ ] Baseline performance measurements (05 §6)
- [ ] Database service roles, transition and posting functions (00 §5); adversarial tests as the real roles
- [ ] Evidence and disposition records; conflicting-evidence handling (00 §2.3)
- [ ] Payout and refund reservation functions with concurrency tests
- [ ] Expo app skeleton consuming the generated API client

## 3. G1 to G5

Each gate delivers a complete, usable slice; dependencies point only backwards.

| Gate | Delivers | Depends on | Key risks |
| --- | --- | --- | --- |
| G1 Payments | Customer onboarding; business workspace, team roles and invitations; approvals; merchant onboarding, KYC review and collection eligibility with limits; conversations with payment request, payment and receipt cards; QR resolve and confirm; MTN collection end to end against its verified contract; receipts and history; notifications for critical events; merchant counter mode; ops console (KYC queue, payment search, cases, approvals); daily reconciliation; payouts per chosen settlement model | G0; MTN production access; KYC rules; mobile client decision; G1 designs (03) | MTN behaviour differs from sandbox; KYC rules late |
| G1+ Provider expansion (sub-gates) | Airtel Money, Zamtel Kwacha and card adapters, each certified independently (05 §3); order set by sandbox scope, access, terms and testability | G1; ADR-010 integration model; ADR-011 card partner; provider agreements | Provider sandboxes differ from production; aggregator capabilities unverified |
| G2 Connections | Contacts and business search; discovery settings; status posts, Discover and moderation (DEC-36); block and report. Text conversations and cards moved to G1 | G1 identities and payments | Abuse and spam; privacy of discovery |
| G3 Merchant tools | Invoices with approved adjustments and allocations (issued invoices never edited); invoice PDFs; point of sale, tills and cash-up (DEC-37); reports; catalogue (scope to confirm). Team roles moved to G1 | G1 payments; G2 for invoice sharing in chat | Invoice/payment status confusion |
| G3 ZRA track (parallel, separate approval) | Eligibility confirmation, VSDC integration design, certification plan | Legal confirmation; ZRA engagement | Certification timeline outside our control |
| G4 Expansion | Voice notes; audio and video calls; conferences; selected partner services | G1 text messaging; ADR-009 build vs buy; cost estimate | Media cost; network quality; partner terms |
| G5 Pilot readiness | BoZ sandbox acceptance and conditions mapped to controls; monitoring, support staffing, onboarding materials, runbooks, restore drill, penetration test, privacy controls, measurable success targets | The approved pilot scope | Support load higher than planned |

**Pilot scope (00 GD-2):** the pilot runs within the BoZ Regulatory Sandbox, subject to acceptance (00 GD-2). Its scope — users, payment methods, limits, duration — is set by the approval. It may start after G1; it need not include every provider or later gate.

## 4. Clean start and legacy retirement

Nothing is migrated: the new system starts with an empty ledger and fresh registrations, and the old system is shut down so it can no longer move money (00 §6).

1. **Now (Phase 0):** rotate or revoke every legacy credential (provider keys, database, deployment, backups).
2. **Phase 0:** list outstanding merchant balances and unresolved payments in the old system; finance agrees how each is settled outside the new ledger.
3. **Before production configuration readiness (00 §8):** disable the old system's payment initiation and revoke its provider credentials, so two systems never move money against the same provider account.
4. **Settle legacy obligations** with finance sign-off; nothing becomes an opening balance in the new ledger.
5. **Archive** old records read-only for the retention period counsel confirms.
6. **Communicate** to existing customers and merchants: re-register and re-verify in the new app.
7. **Decommission** old infrastructure once records are archived.

### Go-live and retirement criteria

- [ ] Every legacy credential rotated or revoked, with a record per credential
- [ ] Old payment initiation disabled and verified by a failed test attempt
- [ ] Legacy obligations settled or assigned, with finance sign-off
- [ ] Old records archived read-only
- [ ] Customer and merchant communication sent
- [ ] Support team briefed for go-live week

**Open:** retention period for archived legacy records (counsel), and how unsettled legacy obligations are handled if a merchant cannot be reached (finance).

## 5. Testing strategy and gate exit criteria

The financial invariants in 04 §3 are tested on every commit; a single failure blocks merge.

### Test layers

| Layer | What it proves | Tooling (proposed) |
| --- | --- | --- |
| Unit | State transitions, money arithmetic, policy rules | `cargo test` |
| Property-based | Any sequence of requests, callbacks and enquiries leaves the ledger balanced with at most one success posting per intent | proptest |
| Concurrency | Parallel duplicate requests, duplicate callbacks and payout reservations never double-post or overdraw | Tokio tests against real PostgreSQL |
| Integration | Repositories and SQL against real PostgreSQL | Testcontainers / CI service |
| Provider simulation | Every row of the 02 §1 failure table: timeouts, declines, missing and duplicate callbacks, forged callbacks, enquiry still pending | Fake provider with scripted scenarios |
| MTN sandbox | Real request/response shapes | Staging, nightly |
| Contract | API matches published OpenAPI | Schema diff in CI |
| Security | Cross-tenant access, auth bypass, rate limits, dependency audit; external pen test before pilot | Test suite + third party |
| Load | SLO targets at agreed multiple of pilot peak | k6 or similar |
| Chaos / recovery | Process kill mid-payment, DB failover, MTN outage | Staging drills |
| Usability | Customers and merchants complete G1 tasks unaided | Moderated sessions |
| Reconciliation | Daily reconciliation produces zero unexplained breaks | Staging rehearsals |

### Gate exit criteria

Every criterion names the evidence a reviewer inspects and the person who accepts it; a gate passes only when every row is accepted.

**G0 Foundation — deterministic integrity proof (simulated provider only)**

Proven by the MVP (§2A) against the simulated provider; these reports are gate 2 evidence for the slice's chunks.

| Criterion | Evidence artifact | Acceptance owner |
| --- | --- | --- |
| I1–I11 enforced in schema and code (00 §5) | Schema diff + passing property and concurrency test reports in CI | Engineering lead |
| F1–F15 handled against the simulated provider, including ambiguous dispatch (F13) | Scenario test report, one test per row | Engineering lead |
| Crash at C1–C5 recovers correctly (00 §3) | Failure-injection test report | Engineering lead + finance |
| Idempotency under concurrent duplicates (create, payout, refund) | Concurrency test report | Engineering lead |
| No secrets in repo or logs; old secrets rotated | Scanner report + rotation record | Security owner |
| Cross-tenant authorization suite passes | Test report covering every endpoint | Security owner |
| Ledger-invariant alerts fire in staging | Alert test record | Operations lead |
| Service roles cannot bypass ledger rules (direct update, delete, unbalanced posting, duplicate effect, illegal transition) | Adversarial test report run as scanpay\_app and scanpay\_worker | Security owner + engineering lead |
| No double refund, double payout or overspend under concurrency | Concurrency test report for reservation functions | Finance |
| Evidence is never rewritten; conflicts open cases | Replay, forgery, duplicate and conflict test report | Engineering lead |
| Accounting model approved | Signed posting examples for every lifecycle step (00 §5) | Finance |

**G1 Payments (MTN) — integration and release-candidate readiness (not authorization to transact; 00 §8)**

Proven by the MVP (§2A) against the MTN sandbox; still no real customer money before G5.

| Criterion | Evidence artifact | Acceptance owner |
| --- | --- | --- |
| MTN Zambia contract confirmed in writing | Signed-off integration notes | Product owner |
| MTN capability matrix confirmed; F1–F15 against MTN sandbox, or documented compensating controls | Adapter certification report (05 §3) | Engineering lead |
| Collections, payouts and refunds end to end | Staging run log with ledger and statement match | Finance |
| Reconciliation daily with zero unexplained breaks over an agreed period | Reconciliation reports | Finance |
| Every state has its designed screen and copy (03 §3) | Design review sign-off | Design lead |
| KYC workflow used on real test applications | Review log | Operations lead |
| Load test meets SLOs set in G0 | Load test report | Engineering lead |
| Usability sessions done; critical findings fixed | Findings log with fixes | Product owner |
| First build slice passes: request from conversation, pending and timeout, verify, receipt, posting, duplicate and late answers | Baseline 2.0 tests PAY-01–PAY-16, FIN-01–FIN-03, EV-03–EV-05 in CI and on staging against the MTN sandbox | Engineering lead + finance |
| Team, approvals and collection controls behave as specified | Organization ORG-01–ORG-70 and Workflow WF-01–WF-43 reports, including real-PostgreSQL concurrency tests | Engineering lead + security owner |
| Collection fails closed with no eligibility decision or no limit policy | ORG-24, ORG-57, PAY-16 reports | Compliance owner |

**G1+ each additional provider (Airtel, Zamtel, card)**

| Criterion | Evidence artifact | Acceptance owner |
| --- | --- | --- |
| Provider contract and agreement confirmed | Signed-off integration notes | Product owner |
| Adapter certification (05 §3) passed | Certification report | Engineering lead |
| Reconciliation proven for that provider | Reconciliation reports | Finance |
| Card data scope confirmed (cards only) | Partner attestation and scope note | Security owner |

**G2 / G3 / G4**

| Criterion | Evidence artifact | Acceptance owner |
| --- | --- | --- |
| Domain flows in 02 §3 pass end to end | E2E test report | Engineering lead |
| Notifications and messages never change financial state | Test report | Engineering lead |
| Invoice status changes only from committed payment events | Test report | Finance |
| G4 media cost estimated and approved; call quality measured in beta | Cost model + beta metrics | Product owner |

**G5 Pilot readiness and activation (BoZ sandbox)**

| Criterion | Evidence artifact | Acceptance owner |
| --- | --- | --- |
| BoZ sandbox acceptance and conditions received | BoZ approval letter | Compliance owner |
| Each condition mapped to a control and tested | Conditions-to-controls matrix with test references | Compliance owner + engineering lead |
| Pilot scope, size, limits and success targets agreed within the approval | Pilot plan with measurement windows, data sources and owners | Product owner |
| Every included feature complete and supportable end to end | Gate records for each included feature | Product owner |
| Dashboards, alerts and on-call live in production | Runbook + alert test record | Operations lead |
| Restore drill passed; runbooks rehearsed | Drill records | Operations lead |
| External penetration test; high and critical findings fixed | Pen-test report + fix evidence | Security owner |
| Support trained; dispute and complaints process tested | Training and test records | Operations lead |
| Initiation switch turned on for the approved cohort only, after every row above is accepted | Audit-log entry with two named approvers (00 §8) | Product owner + compliance owner |

### Requirements traceability (maintained at every gate)

| Requirement group (01) | Flow (02) | Screen (03) | Schema (04) | Control (05) | Test | Gate |
| --- | --- | --- | --- | --- | --- | --- |
| PAY-01–PAY-17 | §1 | §3–4 | §3 | §2–3 | 00 §4–5 suites | G0–G1 |
| LED-01–LED-07 | §1, §4 | §3 | §3 | §3, §6 | Invariant suites I1–I11 | G0 |
| USR-01–USR-06 | §2 | §4 | §2 | §4 | Auth and tenant suites | G0–G1 |
| KYC-01–KYC-06 | §2 | §4 | §2 | §4 | KYC workflow tests | G1 |
| NOT-01–NOT-05 | §3 | §3 | §4 | §6 | Outbox and delivery tests | G1 |
| PMT-01–PMT-05 | §1 | §4 | §3 | §3 | Adapter certification | G1, G1+ |
| REG-01–REG-05 | §1 sandbox steps | §5 | §5 | §4 | Limit and consent tests | G5 |
| CON, INV, MSG | §3 | §4 | §4 | §5 | Domain E2E tests | G2–G4 |
| PAY-20–PAY-23, CNV-01–CNV-03 | §1A | §3–4 | §8 payments, messaging | §5, §7 | PAY, MSG suites | G1 |
| TEAM, ELG | §2, §5 | §4 | Organization 0.4 | §4, §7 | ORG suites | G1 |
| APR | §5 | §4 | Workflow 0.3 | §7 | WF suites | G1 |
| RFD, REC | §4, §6 | §3 | §8 payments, finance | §3, §7 | PAY-30, FIN suites, T09, T14 | G1 |
| POS, STS, SUP | §6, §7 | §4 | §8 retail, messaging, support | §7 | RET, MSG, support suites | G2–G3 |

Acceptance criteria per individual requirement are added to this table as each gate's backlog is refined; a requirement without them cannot close its gate.

### Minimum failure-test matrix (required before financial-path approval)

Each row's test record states its ID, initial conditions, injected failure, expected database state, expected ledger effect, expected customer-visible state and evidence artifact. Rows involving the database, concurrency, roles or transactions run against real PostgreSQL with the real service roles.

| ID | Scenario | Required behaviour |
| --- | --- | --- |
| T01 | Concurrent duplicate create requests | One operation; at most one financial effect |
| T02 | Same key, changed amount or destination | Mismatch rejected; no second operation |
| T03 | Provider accepts, response lost | Stays uncertain; recovered by the provider's verified rule |
| T04 | Worker crash before, during, after dispatch (C1–C2) | No blind new provider operation; no false failure |
| T05 | Query returns pending or not found | No posting and no terminal failure from that result alone |
| T06 | Duplicate, delayed, conflicting callbacks | Deduplicated, evidence kept, no duplicate posting |
| T07 | Later source contradicts applied evidence | Both kept; case opened; no silent rewrite |
| T08 | DB failure after evidence stored, before T4 (C5) | Evidence recoverable; effect wholly committed or not at all |
| T09 | Two refunds race on one payment | Completed + reserved never exceeds refundable amount |
| T10 | Two payouts race on one balance | Reservations never exceed approved available amount |
| T11 | Lease expires while original worker continues | Stale writes rejected by fencing token and state checks |
| T12 | App disconnects after initiation | Resume fetches the existing operation; never auto-creates another |
| T13 | Cross-tenant access, including altered session context and forged actor | Denied |
| T14 | Statement missing, duplicated, late or mismatched | Reconciliation exception with documented resolution |
| T15 | Backup restore or database failover | Authoritative state, pending work and evidence recovered; no duplicate effects |

## 6. Team, risks, decisions and cross-references

### Roles needed (headcount to be set after Phase 0)

| Role | Focus |
| --- | --- |
| Product owner | Scope, gate approvals, pilot targets |
| Engineering lead / architect | ADRs, ledger and payment design, code review |
| Rust backend engineers | Domain modules, provider adapter, workers |
| React Native (Expo) engineers | Customer and merchant apps |
| Designer | 03 deliverables and usability testing |
| QA / test engineer | Provider simulation, regression, load |
| Operations / support lead | Ops console requirements, runbooks, pilot support |
| Legal / compliance (part-time or advisor) | Regulatory confirmation, BoZ sandbox application and conditions, KYC rules, privacy |
| Finance | Reconciliation sign-off, settlement model |
| Independent architecture reviewer | Consistency and security review of each chunk before freeze; verification of evidence (gate 3) |

### Delivery risks

| Risk | Effect | Mitigation |
| --- | --- | --- |
| Rust experience on the team is limited | Slower G0 | Pairing, early spike, external review of ledger code |
| MTN production access or terms delayed | G1 blocked | Start partner engagement in Phase 0; fake provider keeps work moving |
| Legacy obligations unsettled at go-live | Merchant disputes; two systems moving money | List and settle in Phase 0; disable old payment initiation before go-live |
| Scope pressure to open later gates early | Integrity risk | Gate rule in 01 §6; flags off until exit criteria pass |
| Regulatory requirements broader than expected | Rework | Legal check in Phase 0 |
| BoZ sandbox acceptance delayed or conditions narrower than planned | Pilot slips or shrinks | Prepare the application in Phase 0; limits and scope are configuration |
| Additional providers harder to access than MTN | G1+ sub-gates slip | Sub-gates independent of G2–G3; aggregator option evaluated in parallel |

### Decision log (to maintain)

| Date | Decision | Made by | Reference |
| --- | --- | --- | --- |
| 2026-10-18 | Full scope, payments first, Rust backend, payment integrity as first gate | Product leadership | 01 |
| 2026-10-18 | GD-1: multi-provider platform; MTN first, then Airtel Money, Zamtel Kwacha, cards | Product leadership | 00 |
| 2026-10-18 | GD-2: pilot intended within the BoZ Regulatory Sandbox, subject to acceptance | Product leadership | 00 |
| 2026-10-18 | GD-3: greenfield rebuild; old code and data discarded, not migrated | Product leadership | 00 |
| 2026-10-18 | GD-4: React Native with Expo; Infratel Zambia hosting (subject to confirmation) | Product leadership | 00, 05 |
| 2026-10-21 | Architecture work starts before full UI sign-off; Android render and accessibility passes run in parallel | Product owner | Baseline 1.0 |
| 2026-10-21 | Point of sale and status updates are in scope | Product owner | 01 §2 |
| 2026-10-21 | PostgreSQL self-managed by Scanpay | Product owner | 05 ADR-013 |

### Open-decision register

No decision that affects financial correctness may still be open when the matching implementation is approved. By Phase 0 exit, every row has a named owner, a deadline or gate dependency, and a stated consequence if it slips. A decision may stay open only if it does not block the next stage. It closes when it links to the signed decision, the implementation that enforces it and the evidence that proves it.

| Decision | Accountable owner | Latest decision point | Status |
| --- | --- | --- | --- |
| Ledger accounting and settlement model | Finance + engineering | Before ledger schema is frozen | Open |
| Canonical state and evidence rules | Product + engineering + finance | Before financial-path implementation | Drafted in 00 v1.1 |
| MTN Zambia API, callbacks, queries, statements | Engineering + partnerships | Phase 0 / before G1 certification | Open |
| Direct integration vs aggregator | Product + engineering + partnerships | Phase 0 | Open |
| Database enforcement and tenant isolation | Engineering + security | Before G0 passes | Drafted in 00 §5 |
| Authentication and recovery policy | Product + security | Before G0 passes | Drafted in 05 §4 |
| KYC requirements and merchant eligibility | Compliance + operations | Before G1 onboarding | Open |
| Infratel hosting arrangement, backup and DR targets | Engineering + operations | Before production architecture approval | Selected; confirmation open |
| Refund policy, fees, manual exceptions | Product + finance + operations | Before G1 | Open |
| Legacy obligations settlement and record retention | Finance + counsel | Phase 0 | Open |
| BoZ conditions, limits, pilot success targets | Compliance + product | Before pilot approval | Open |
| Card partner and card-data scope | Partnerships + security | Before card sub-gate | Open |
| G4 media build vs buy | Product + engineering | Before G4 | Open |
| No live activation before authorization; G1/G5 meanings (00 §8) | Product + compliance + operations | Before release criteria are approved | Drafted in 00 v1.2 |
| Statement import, reconciliation completeness, exception resolution | Finance + engineering + operations | Before G1 passes | Open |
| SLOs, alert thresholds, runbooks, on-call ownership | Engineering + operations | Targets in G0; proven before G1 and pilot | Open |
| ZRA eligibility and certification path | Compliance + product | Before any compliance claim | Open |
| Architecture decisions DEC-23–DEC-37, C1–C6, N1–N13 (full register: Baseline 2.0 §15) | Owners per Baseline §15.3; named individuals to be assigned by the product owner | Before the chunk each one blocks is frozen (§2A) | Open |
| One vocabulary for payment states and refund vs reversal (N13) | Product + finance | Before chunk 6 freeze and G1 copy freeze | Open |
| Approver for refunds in sole-Owner businesses (N1) | Product + security | Before chunk 4 freeze; blocks refunds for most pilot merchants | Open |

### Cross-references

01 PRD (requirements, gates, success measures) · 02 Application Flow (scenarios to test) · 03 Design Brief (design deliverables) · 04 Backend Schema (invariants, migration mapping) · 05 TRD (stack, security, operations, ADRs).
