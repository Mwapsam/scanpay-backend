# Scanpay 01 — Product Requirements Document

Oct 18, 2026 · @TCS

## Document control

This PRD defines what Scanpay must do; it is the parent of the other five rebuild documents and is a draft for review, not a statement of what exists today.

| Field | Value |
| --- | --- |
| Document | 01 of 6 — Product Requirements Document |
| Version | 1.2 draft (21 Oct 2026: aligned with the approved UI and Architecture Baseline 2.0) |
| Audience | Business executives and the engineering team |
| Scope | Full product: fourteen capability areas (§2), built on ten technical domains (Baseline 2.0 §5) |
| Initial priority | Payments; MTN Mobile Money first, then Airtel Money, Zamtel Kwacha and cards (00 GD-1) |
| Backend direction | Rust-first, PostgreSQL |
| First release gate | Payment integrity (G0) |
| Status | Proposed for review |

**Governing contract.** Money movement, states, evidence and recovery are defined once in **00 Product and Financial Integrity Contract**; this PRD may not redefine them. Four governing decisions apply to the whole set. Scanpay is a multi-provider platform (GD-1). The controlled pilot is intended to run within the Bank of Zambia Regulatory Sandbox framework, subject to acceptance and its conditions (GD-2). Scanpay is a greenfield rebuild with no legacy code or data carried forward (GD-3). The stack is a Rust backend, a React Native (Expo) app, a PostgreSQL database that Scanpay runs itself (decided 21 Oct 2026), and Infratel Zambia hosting, subject to confirmation (GD-4). Requirements here describe the new product, not legacy behaviour.

**What changed in 1.2.** Since 1.1 the full UI has been designed and approved for architecture work, and the architecture has been drafted. This version brings the requirements into line with both:

- New capability areas: business workspace and team roles, approvals, collection eligibility and limits, refunds and reversals, point of sale and cash, reconciliation, status posts and Discover, support and disputes (§2, §3.11–3.18).
- Text conversations with payment cards move into the first release, because the first build slice requests payment from a conversation (§6).
- Payment status labels follow the approved UI (PAY-05). Issued invoices are never edited (INV-01). KYB approval no longer implies a business may collect (KYC-05).
- PostgreSQL is self-managed by Scanpay.
- 21 Oct 2026: consistency fixes (KYC-03 states aligned with Organization 0.4; duplicate priority note removed).
- 21 Oct 2026: G0 and G1 are proven by the MVP, the first build slice (§6, 06 §2A).

Companion documents: [Architecture Baseline 2.0](https://claude.ai/artifact/3TaqDBgNAXW9Y8P6cUW3xS) holds the technical domains and the full decision register; [Organization Draft 0.4](https://claude.ai/artifact/K1jPBwUKMcLFrJxCxbaoRF) and [Workflow Draft 0.3](https://claude.ai/artifact/7Re3AyWYUhyf6Z4fjAbE72) detail two domains; the [approved UI](https://claude.ai/artifact/1r2Uypv2kjvkZugFCGZXyk) shows every screen. Where this PRD and those documents disagree, raise it; neither silently wins.

**How to read this set.** Executives can read sections marked *Business summary* in each document. Engineers should read everything. Every document uses the same labels:

- **Requirement** — agreed behaviour the product must have.
- **Assumption** — a working belief that must be validated; each has an owner in the decisions table.
- **Open decision** — not yet chosen; recorded so it is not silently assumed.
- **Planned** — intended capability; nothing in this set claims a planned feature is built, tested or live.

**The six documents**

1. 01 PRD — what and why (this document)
2. 02 Application Flow — journeys, states, failures and recovery
3. 03 Design Brief — brand, UX, accessibility and screens
4. 04 Backend Schema — domain boundaries, PostgreSQL entities, ledger invariants
5. 05 TRD — Rust architecture, APIs, security, integrations, operations
6. 06 Implementation Plan — phases, migration, testing, gates

## 1. Executive summary and vision

*Business summary.* Scanpay is to become a Zambia-first platform where customers pay businesses and the payment becomes the start of an ongoing relationship: **Pay. Connect. Grow.** We rebuild in gated stages, starting with payments that are provably correct, because trust in every transaction is the foundation for everything else.

Customers will pay businesses, keep useful records, communicate with businesses and access services. Businesses will receive payments, manage customers, issue invoices, monitor activity and adopt further digital tools over time.

The backend is rebuilt in Rust. The first engineering priority is payment correctness, security, ledger integrity, reliable provider integration and transaction recovery — not raw request throughput.

### Vision

Make digital payments more useful by connecting each transaction to the relationships, records and tools that customers and businesses need to transact and grow.

### Product principles

1. **Correctness before speed.** A fast incorrect payment is a failure.
2. **One financial truth.** Every posted financial movement is traceable and auditable.
3. **Explicit transaction states.** A timeout or missing callback never automatically means a payment failed.
4. **Security by design.** Authentication, authorization, tenant isolation and sensitive-data protection are foundational.
5. **Useful connections.** Messaging, contacts and notifications serve real customer–business workflows.
6. **Progressive delivery.** A feature ships only when its dependencies and acceptance criteria are met.
7. **Verified claims.** Marketing and executive reporting distinguish implemented, tested, pilot-validated and planned capabilities.

### Business objectives

- Establish a trustworthy payment experience for customers and merchants.
- Reduce uncertainty when a payment is initiated, completed, delayed or disputed.
- Give customers and businesses a reliable record of every transaction.
- Increase repeat transactions and ongoing customer–business relationships.
- Introduce merchant tools with a clear path to sustainable revenue.
- Expand payment methods and services only when commercially and operationally justified.

## 2. Users and scope

**Business summary.** All fourteen capability areas are in scope. The first controlled payments release needs payments, ledger, user management, the business workspace and team roles, business KYC, collection eligibility, approvals, conversations with payment cards, and critical notifications.

### Target users

| User group | What they need from Scanpay |
| --- | --- |
| Customers | Pay businesses, confirm who they are paying, get receipts, review history, save contacts, talk to businesses |
| Merchants and business owners | Receive payments, monitor collections, issue invoices, track records, manage customers and staff |
| Business staff and finance operators | Act within granted permissions, review transactions, reconcile discrepancies, support customers |
| Scanpay operations and support | Investigate payment issues, review audit trails, run onboarding and KYC review, monitor platform health |

**Assumption:** these are intended groups, not evidence of an existing user base. User interviews and the pilot must confirm their needs.

### Capability areas

| Area | Intended capability | Priority | Technical domain (Baseline 2.0) |
| --- | --- | --- | --- |
| Payments | QR payments, recipient confirmation, payment requests, provider confirmation, receipts, recovery | P0 | `payments` |
| Ledger and reconciliation | Financial accounts, immutable postings, reversals, statement matching, reconciliation cases | P0 | `finance` |
| User management | Registration, sign-in, devices, sessions, step-up, account security | P0 | `shared` |
| Business workspace and team | Business accounts, members, four roles, invitations, permissions | P0 | `organization` |
| Business KYC | Evidence collection, review workflow, verification status | P0 for merchant onboarding | `organization` |
| Collection eligibility and limits | Whether a business may collect, restrictions, limits | P0 | `organization` (policy), `payments` (enforcement) |
| Approvals | Independent second approval for sensitive actions | P0 | `workflow` |
| Conversations | Text chat between customers and businesses, with invoice, payment and receipt cards | P0 for the first slice (text and cards only) | `messaging` |
| Notifications | Payment and account events, delivery tracking, preferences | P0 for critical events | `notifications` |
| Refunds | Merchant-initiated refunds, approved and provider-confirmed | P0 (G1); blocked for MTN until its refund API is confirmed | `payments` |
| Contacts and invoicing | Saved contacts, verified identities, commercial invoices | P1 | `commerce` |
| Point of sale and cash | Tills, sales, tenders, returns, cash-up | P1 | `retail` |
| Status posts, Discover, support | Business and user status updates, discovery, moderation, disputes and support cases | P2 (status, Discover); P1 (support) | `messaging`, `support` |
| Voice notes, calls and conferences | Audio and video communication | P2, staged | `messaging` (to be designed) |

P0 = required for the first controlled payments release (G1). P1 = next functional expansion (G2–G3). P2 = needs extra infrastructure or policy (G4). The priorities of the new areas are **Proposed** and need product confirmation. See section 6 for gates.

### Explicit exclusions from the first payments release

- Treating an unverified provider callback as proof of payment.
- Marking a payout complete before the provider confirms it.
- Treating an unresolved timeout as a definite failure.
- Advertising messaging, video, invoicing or third-party services before they are built and accepted.
- Claiming ZRA Smart Invoice compliance before eligibility, integration and certification are established.
- Enabling Airtel Money, Zamtel Kwacha or card payments for users before each passes its own integration gate.

## 3. Functional requirements

Requirement IDs (PAY-01, LED-01…) are referenced by 02–06. Each requirement's priority is its domain priority (§2); its acceptance criteria, verification method, evidence and owner are recorded in the traceability table in 06 §5, which must be complete for a requirement before its gate closes.

### 3.1 Payments (P0)

**Customer**

- PAY-01 Scan a merchant QR code or otherwise identify a merchant.
- PAY-02 See the resolved merchant's verified identity before authorizing.
- PAY-03 Enter or confirm the amount and see fees and transaction details.
- PAY-04 Initiate a payment with a supported method: MTN Mobile Money first; Airtel Money, Zamtel Kwacha and cards as each passes its gate.
- PAY-05 See a clear result: requested, pending confirmation, awaiting resolution, completed, failed, voided or reversed. Pending confirmation and awaiting resolution are never shown as paid or failed (labels proposed; C5, C6 Open).
- PAY-06 Get a receipt and see transaction history.
- PAY-07 Retry safely when the outcome is uncertain, with no duplicate financial effect.
- PAY-08 Report a disputed or missing payment.

**System**

- PAY-10 Assign a unique internal payment ID and require an idempotency key.
- PAY-11 Validate merchant status, limits, amount and authorization.
- PAY-12 Authenticate and validate provider notifications using the provider's supported mechanisms.
- PAY-13 Persist provider references and every state transition.
- PAY-14 Post ledger entries exactly once, logically, even when requests or callbacks repeat.
- PAY-15 Reconcile internal records against provider records.
- PAY-16 Support investigation and recovery of unresolved payments.
- PAY-17 Keep an auditable history of status changes and administrative actions.
- PAY-18 Merchant QR codes carry a high-entropy, revocable reference (not an account number); a revoked code or an ineligible merchant blocks payment before confirmation. A QR code identifies a merchant; it is never proof of payment.
- PAY-19 Merchant counter mode shows only payments committed in T4, the same state as the receipt and ledger.

**Required distinction.** A payment request, a provider-confirmed collection, a ledger posting and a merchant payout are four separate concepts with separate state machines (02 §1, 04 §3).

**Added in 1.2:**

- **PAY-20** A business member requests a payment from a customer conversation; the request appears in the chat as a card that updates with the payment's state.
- **PAY-21** A payment request may be linked to an invoice, a point-of-sale sale, or stand alone. A standalone request records the customer, purpose, amount, currency, the member who made it and a unique idempotent reference (N10 Open; proposed yes).
- **PAY-22** A business cannot create a payment request unless it is currently eligible to collect, no restriction blocks it, and the amount is within its limits; when any of these cannot be established, the request is refused (Baseline R10).
- **PAY-23** One open payment request per invoice, and one electronic tender per sale (Baseline R2).

### 3.2 Ledger and wallets (P0)

- LED-01 Keep authoritative double-entry records for every supported account type.
- LED-02 Every journal entry balances; every posting is traceable to its cause.
- LED-03 Derive or maintain balances by one defined strategy (04 §3).
- LED-04 Prevent concurrent requests from overspending or duplicating effects.
- LED-05 Reverse with compensating entries; never rewrite posted history.
- LED-06 Separate customer collections, wallet movements, merchant payouts and settlement.
- LED-07 Restrict privileged adjustments; dual approval and audit for each.

A displayed balance is never a substitute for the underlying financial record.

### 3.3 User management (P0)

- USR-01 Customer and business registration.
- USR-02 Secure authentication and session management.
- USR-03 Role-based and resource-level authorization.
- USR-04 Account recovery, session revocation and security notifications.
- USR-05 Ownership checks and tenant isolation on every request.
- USR-06 Step-up verification for sensitive financial and administrative actions.

**Open decision:** authentication factors and recovery rules (05 §4).

### 3.4 Business KYC (P0 for merchant onboarding)

- KYC-01 Collect required business and representative information.
- KYC-02 Store verification evidence securely with restricted access.
- KYC-03 Track KYB case states: draft, submitted, in review, more information needed, approved, rejected, withdrawn (Organization 0.4). Stopping a business from collecting is a restriction (ELG-03), not a KYB state.
- KYC-04 Record reviewer, decision, reason and timestamp.
- KYC-05 Keep KYB approval separate from permission to collect: approving a business's KYB never by itself lets it collect. Eligibility, restrictions and limits are their own decisions, each needing a second staff approver (§3.13; DEC-34 Open).
- KYC-06 Support periodic review and changes to business details.

**Assumption:** document requirements and retention periods must be validated against Zambian obligations and MTN's merchant arrangements.

### 3.5 Notifications (P0 for critical events)

- NOT-01 Deliver critical payment-status and security notifications by push and in-app.
- NOT-02 Track delivery attempts and failures where the channel allows.
- NOT-03 Never treat notification delivery as proof of a financial event.
- NOT-04 Keep sensitive details off lock screens.
- NOT-05 Use durable, retryable event delivery for critical notifications.

### 3.6 Contacts (P1)

- CON-01 Find and save customer or business identities.
- CON-02 Resolve recipients by verified identifiers and merchant records.
- CON-03 Distinguish verified businesses from unverified names.
- CON-04 Respect discovery permissions and privacy settings; never expose a user's contact list.

### 3.7 Invoicing (P1, after payment integrity)

- INV-01 Create and edit draft invoices, and issue them. An issued invoice is never edited: changes are adjustments or cancellations, recorded as new records and approved by an Owner (DEC-25 Open).
- INV-02 Record line items, quantities, prices, taxes and totals.
- INV-03 Track draft, issued, partially paid, paid and cancelled. Overdue is a flag on an issued invoice, not a separate state.
- INV-04 Link payments to invoices without conflating invoice and payment status.
- INV-05 Produce shareable invoice documents and receipts.
- INV-06 Preserve history of material changes.

**Tax distinction.** A Scanpay commercial invoice is not a certified ZRA Smart Invoice. ZRA eligibility, VSDC integration and certification are a separate workstream (06 §3).

### 3.8 Messaging and calling (text and payment cards P0; voice and calls P2)

- MSG-01 Text conversations between authorized users.
- MSG-02 Voice notes: record, upload, deliver, play.
- MSG-03 One-to-one audio calls.
- MSG-04 One-to-one video calls.
- MSG-05 Multi-party conference calls.
- MSG-06 Conversation membership and access control.
- MSG-07 Delivery and read states.
- MSG-08 Call states: inviting, ringing, accepted, declined, ended, failed; reconnection.
- MSG-09 Report, block and abuse controls.
- MSG-10 Microphone, camera and notification permission handling.

Signaling, WebRTC, TURN/STUN, media providers, bandwidth cost and retention are decided in 05 §5.

### 3.9 Payment methods (GD-1)

- PMT-01 One shared payment domain owns states, idempotency, ledger posting, audit, reconciliation and recovery for every method (00 §2–4).
- PMT-02 Each provider is reached through its own adapter that respects that provider's verified contract.
- PMT-03 Each method is enabled per environment and per user group by configuration, only after its gate evidence passes.
- PMT-04 Customers see only methods enabled for them; unavailable methods are not advertised.
- PMT-05 Card payments follow the same lifecycle; card data is handled by the selected acquirer or processor so Scanpay stays out of card-data scope where possible (to confirm when the partner is chosen).

### 3.10 Regulatory and sandbox constraints (GD-2)

- REG-01 Pilot activities, users, payment methods, transaction limits and duration are limited to what BoZ approves for the sandbox.
- REG-02 Limits and eligibility rules are configurable and enforced server-side (00 I11).
- REG-03 Customer disclosures, consent, complaints handling and escalation follow BoZ conditions once specified; no regulatory wording is invented in advance.
- REG-04 Reporting required by BoZ is producible from Scanpay's records.
- REG-05 Sandbox participation is not assumed to remove licensing, partnership, consumer-protection, AML/CFT, data-protection or tax obligations; each is confirmed with BoZ and qualified Zambian counsel.

### 3.11 Conversations as the business record (P0 for the first slice)

- **CNV-01** Every conversation, invoice, payment, receipt and ledger entry for a customer links back to the same business and, where it started in chat, the same conversation. A report line can be traced to its origin.
- **CNV-02** Invoice, payment and receipt cards in a conversation show the authoritative state from payments; a card never shows paid before the payment is verified, and an older update never overwrites a newer one.
- **CNV-03** A business conversation can be assigned to a member; who sent each business message is recorded (DEC-32 Open: acting as yourself or as the business).

### 3.12 Business workspace, team and roles (P0)

- **TEAM-01** Four roles: Owner, Finance, Cashier, Viewer, as drawn in the approved UI (DEC-31 Open; proposed). Custom roles are deferred.
- **TEAM-02** Every action is authorized on the server against the member's current role. Hiding a button is not enforcement.
- **TEAM-03** Invitations are sent by SMS, expire (proposed 7 days, N6 Open), and can be accepted only by the person who owns the invited phone number.
- **TEAM-04** A business always has at least one active Owner. The last Owner cannot leave, be removed or be demoted until another Owner exists.
- **TEAM-05** Promoting someone to Owner or Finance needs another Owner's approval and step-up verification.
- **TEAM-06** Scanpay staff use a separate operations console and sign-in; staff are never business members.

### 3.13 Collection eligibility, restrictions and limits (P0)

- **ELG-01** Whether a business may collect is an explicit decision recorded by compliance staff and approved by a second staff member. With no decision, the business cannot collect.
- **ELG-02** Collecting while KYB is still under review is off by default (DEC-34 Open).
- **ELG-03** Risk staff can pause a business's collections, with a reason and a second approver. A pause blocks new requests; payments already in progress still complete or resolve normally. Lifting a pause does not by itself allow collection if another requirement is unmet. Emergency pauses before approval: N2 Open.
- **ELG-04** Limits per transaction, per day, per month and in total, by payment method where needed. Values are configuration (DEC-35 Open). If no applicable limits are defined, collection is refused.
- **ELG-05** Changes to eligibility, restrictions and limits can be scheduled ahead and cancelled before they start; their full history is kept, so any past decision can be reproduced.

### 3.14 Approvals (P0)

- **APR-01** Sensitive actions need an independent approver: refunds, invoice adjustments and cancellations, reconciliation decisions, promotions to Owner or Finance, and every staff eligibility, restriction, limit and settlement-account change.
- **APR-02** Nobody approves their own request, including through a second account or role.
- **APR-03** An approval authorizes one exact action, once, within a time window. Approving never moves money: the UI shows "Approved · waiting for MTN" until the provider confirms.
- **APR-04** Thresholds, expiry times and which actions need step-up are configurable policy (DEC-23 Open).
- **APR-05** A business with a single Owner has no second approver for its own refunds (N1 Open). Until N1 is decided, such refunds cannot be approved.

### 3.15 Refunds and reversals (P0, G1)

- **RFD-01** A **refund** is money a business chooses to send back to a customer. A **reversal** is money the provider takes back and reports to Scanpay (C1 Open; proposed wording).
- **RFD-02** Several partial refunds are allowed up to the verified payment amount; refunds in progress count against that amount.
- **RFD-03** A refund that times out stays pending until the provider answers; silence never marks it refunded or failed.

### 3.16 Point of sale and cash (P1)

- **POS-01** A cashier opens a till, rings up sales and takes cash or MTN payment; each sale has at most one electronic payment.
- **POS-02** A sale waiting on an MTN payment is held: it cannot be abandoned, and its till cannot close until the payment is resolved.
- **POS-03** Cash cannot be taken for a sale while its MTN payment is still unresolved (DEC-37 Open; proposed).
- **POS-04** Cash-up records the counted cash and any difference. Float rules, acceptable differences and cash refunds are DEC-37 Open.
- **POS-05** Returns with a refund go through approval (§3.14).

### 3.17 Reconciliation (P0, with the ledger)

- **REC-01** Import provider statements and match each line to a payment or refund.
- **REC-02** A mismatch opens a reconciliation case. One Finance member proposes a decision and another approves it (an Owner may approve when no Finance member exists: N3 Open).
- **REC-03** A statement line is evidence only: it never itself marks a payment completed or failed (C3 Open; proposed).
- **REC-04** Payment operations staff may close an unresolved payment from the provider's written answer, with a second staff member approving (C2 Open; proposed).

### 3.18 Status posts, Discover and support (P1–P2)

- **STS-01** Businesses, and users where allowed, post status updates that their contacts can see; the poster chooses how long a post stays up.
- **STS-02** Posts can be reported; reported posts go to a moderation queue. Discover ranking, boosting and moderation rules are DEC-36 Open.
- **SUP-01** Customers and businesses can open support cases and payment disputes; staff work them in the operations console. A dispute never itself changes a payment's state.

## 4. Non-functional requirements

Numeric targets are set from baseline measurement, provider constraints and load tests (06 §5) — none are invented here.

| Area | Requirement |
| --- | --- |
| Correctness | Duplicate requests and callbacks never create duplicate postings. |
| Availability | Service-level objectives defined for payment initiation, status lookup and other critical APIs. |
| Performance | p95 and p99 latency targets per API category; provider latency measured separately. |
| Security | Managed secrets, strong authentication, least privilege, encryption in transit and at rest, audit trails. |
| Consistency | A defined database transaction and concurrency control for every financial state transition. |
| Recovery | Safe recovery from timeouts, crashes, duplicate events and unavailable dependencies. |
| Observability | Requests, provider references, postings and jobs correlated by ID; no secrets in logs. |
| Privacy | Data minimisation; retention and deletion policies per data class. |
| Maintainability | Explicit domain boundaries, documented APIs, automated tests, reproducible builds. |
| Scalability | Scale components on observed bottlenecks, not theory. |

### Rust-first performance strategy

Rust gives efficient execution, memory safety and strong concurrency, but it does not by itself make the platform fast. End-to-end speed also depends on database design, locking, network latency, provider response time, queues and real-time connections. The rebuild therefore:

- Uses Rust for the backend and all core domain logic.
- Chooses the async runtime and web framework in 05 §1.
- Uses PostgreSQL as the authoritative datastore.
- Benchmarks database access, payment workflows and concurrent traffic.
- Never trades financial correctness or recovery guarantees for speed.
- Starts as a modular monolith; no microservices for theoretical gains.

## 5. Success criteria

*Business summary.* We measure verified outcomes, not features shipped. Numeric targets are set before the pilot, each with a measurement window, data source and owner, and within any limits set by the BoZ sandbox approval. No adoption, revenue, conversion or performance figures are assumed in this document.

| Category | Measures |
| --- | --- |
| Payment reliability | Provider-confirmed success rate, duplicate-posting incidents (target: zero), age of unresolved payments, reconciliation discrepancies |
| Customer experience | Time to confirmed outcome, receipt availability, recovery of failed payments, repeat usage |
| Merchant value | Active merchants, transactions per active merchant, repeat customers, invoicing adoption |
| Operational readiness | Incident frequency, time to detect and resolve, support load, reconciliation completion |
| Commercial sustainability | Revenue per active merchant, provider costs, support costs, contribution margin |

## 6. Release gates

The scope is full; delivery is gated by risk and dependency. Finishing a feature does not open the next gate — its tests, operational controls, security requirements and acceptance criteria must also pass. Detailed exit criteria are in 06 §5.

1. **G0 Foundation — payment integrity and security.** Shared payment domain, provider abstraction, ledger invariants, idempotency, concurrency, auditability and crash recovery proven deterministically against a simulated provider (00 §3–5).
2. **G1 Payments (MTN) — first provider end to end.** Scan, confirm recipient, initiate, verify outcome, post once, receipt, recover uncertain outcomes, payouts as approved and reconciliation, against MTN's verified contract. \*\*G1+ Provider expansion\*\* then adds Airtel Money, Zamtel Kwacha and cards as independent sub-gates through the same architecture, ordered by sandbox scope, provider access, commercial terms and testability.
3. **G2 Connections — contacts and engagement.** Saved contacts, status posts and Discover, built on verified identities. Text conversations with payment cards move into G1, because the first build slice requests payment from a conversation.
4. **G3 Merchant tools — invoicing and business operations.** Commercial invoices, catalogue and orders, point of sale and cash-up, reporting; separate ZRA path. Team roles and approvals are needed earlier, in G1.
5. **G4 Expansion — services and advanced communications.** Selected services; voice notes, audio, video and conferences when infrastructure and cost are justified.
6. **G5 Pilot — controlled pilot readiness.** Within the BoZ Regulatory Sandbox: application accepted, conditions mapped to controls, and monitoring, support, onboarding, recovery runbooks, privacy controls and measurable release criteria verified for the approved pilot scope.

G0 and G1 are proven by the MVP, the first build slice (06 §2A); no real customer money moves before G5.

The pilot scope is whatever BoZ approves within the sandbox. The pilot is what G5 authorizes, so it begins only once G5 has passed for that scope; G5 may follow G1 directly (06 §1), so the pilot can start right after G1 if BoZ has accepted the application. It need not include every payment method or later gate. Every included feature must be complete and supportable end to end; users are never exposed to half-built functionality.

## 7. Business model hypotheses

These are hypotheses to test, not established revenue:

- Transaction-linked revenue, where provider and commercial agreements allow.
- Merchant subscriptions for advanced tools.
- Premium reporting, invoicing and staff management.
- Partner services integrated into the platform.

Each is tested against merchant willingness to pay, provider agreements, operating cost, regulation and competing alternatives.

## 8. Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Duplicate or incorrect payments | Idempotency, ledger invariants, concurrency tests, reconciliation |
| Provider timeouts or missing callbacks | Explicit pending states, status enquiries, recovery jobs |
| Exposed credentials | Secret rotation, managed secret store, restricted access |
| Over-broad first delivery | Release gates, feature flags, bounded first release |
| Costly messaging infrastructure | Staged delivery, media cost estimates, capacity tests |
| Wrong regulatory assumptions | Validate with authorities and MTN before launch |
| Old system keeps moving money after the new one goes live | Revoke legacy credentials now; disable old payment initiation before go-live (00 §6) |
| Performance work breaking correctness | Financial invariants as non-negotiable test conditions |
| Sandbox not accepted, or conditions narrow the pilot | Engage BoZ early in discovery; configurable limits; pilot plan adaptable to approved scope |
| Provider behaviour differs between MTN, Airtel, Zamtel and cards | Shared domain with per-provider adapters; each passes the 00 §4 failure matrix before enablement |

## 9. Assumptions and open decisions

| Decision | Proposed starting point | Status |
| --- | --- | --- |
| Initial geography | Zambia | Assumption |
| Initial provider | MTN Mobile Money first, subject to current integration and commercial terms; Airtel Money, Zamtel Kwacha and cards follow (GD-1) | Assumption — confirm |
| Backend language | Rust | Decided |
| Primary datastore | PostgreSQL | Proposed |
| Architecture | Modular monolith with separable domain boundaries | Proposed |
| Mobile app | New React Native app with Expo, Android and iOS (GD-4) | Decided |
| Legacy data | None migrated; users re-register; old obligations settled outside the new ledger (GD-3, 00 §6) | Decided |
| Hosting | Infratel Zambia, subject to confirming arrangement, data location, security controls, backups and DR (GD-4) | Selected — confirmation pending |
| ZRA integration | Confirm eligibility and certification before live tax submission | Open |
| Launch targets | Set from pilot size, baselines and business constraints | Open |
| Integration model | Evaluate direct integrations vs a payment aggregator in discovery; Instacom Payment Solutions was previously considered but is not selected or verified | Open — blocks G1+ |
| Card acquirer or processor | Select partner; confirm card-data scope | Open — blocks card sub-gate |
| Pilot regulatory pathway | BoZ Regulatory Sandbox, subject to acceptance and conditions (GD-2) | Decided as intent; acceptance pending — blocks G5 |
| Pilot scope | Set by approved sandbox scope; earliest after G1 | Open — owner: product + compliance |
| Database operation | PostgreSQL run by Scanpay itself, not a provider-managed service | Decided 21 Oct 2026 |
| Business roles (DEC-31) | Owner, Finance, Cashier, Viewer; custom roles deferred | Open — product + security |
| Collecting during KYB review (DEC-34) | Off by default | Open — product + compliance |
| Limit structure and values (DEC-35) | Per transaction, daily, monthly, cumulative; values as configuration; refuse when undefined | Open — product + compliance |
| Approval policy and sole-Owner businesses (DEC-23, N1) | Versioned policies; sole-Owner fallback only if explicitly approved | Open — product + security; N1 blocks refunds for most pilot merchants |
| Refund vs reversal wording; payment labels (C1, C5, C6) | As in RFD-01 and PAY-05 | Open — product |
| Till, float, cash differences, cash refunds (DEC-37) | Block cash while an MTN payment on the sale is unresolved | Open — product + finance |
| Status expiry, Discover, moderation (DEC-36) | Separate policies before G2 | Open — product |
| Chart of accounts; whether Scanpay holds funds (DEC-01, DEC-18) | Evaluate operating without holding customer funds first | Open — finance + regulatory |

**Dependencies:** written provider contracts (MTN first), completed provider capability matrices (00 §4), finance sign-off on the ledger and settlement model, Infratel hosting confirmation; confirmation of MTN callback authentication and status-enquiry support; applicable KYC, privacy, payments and tax requirements; an operating budget, pilot scope and support capacity.

## 10. Acceptance criteria for this PRD

- [ ] All capability areas (§2) and their priorities agreed, including the 1.2 additions.
- [ ] The first payments release explicitly bounded.
- [ ] Financial and security invariants accepted by business and engineering.
- [ ] Release gates and success measures agreed.
- [ ] Every regulatory and provider assumption has a named owner.
- [ ] Remaining decisions recorded, not silently assumed.

## 11. Cross-references

- **00 Product and Financial Integrity Contract** — canonical states, commit boundary, provider failure matrix, invariants; governs 01–06.

* **02 Application Flow** — journeys, transaction states, retries, failures, recovery.
* **03 Design Brief** — brand, UX, accessibility, screens.
* **04 Backend Schema** — entities, ledger invariants, constraints, data ownership.
* **05 TRD** — Rust architecture, APIs, security, performance, integrations.
* **06 Implementation Plan** — sequence, testing, dependencies, gate criteria.
