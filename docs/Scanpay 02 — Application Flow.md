# Scanpay 02 — Application Flow

Oct 18, 2026 · @TCS

## Purpose, actors and conventions

This document defines every Scanpay journey, step by step, including what happens when things go wrong; it is the behavioural contract between the PRD (01) and the schema (04) and APIs (05).

**Governed by 00.** States, transitions, evidence rules and recovery come from the 00 Product and Financial Integrity Contract; this document shows them as journeys and may not redefine them. Flows are written provider-neutrally: MTN Mobile Money is the first adapter (G1); Airtel Money, Zamtel Kwacha and cards use the same flows once their adapters pass their gates. Pilot flows are subject to BoZ sandbox conditions (00 GD-2). These are the journeys the new system must implement from scratch; no legacy behaviour is assumed (00 GD-3).

*Business summary.* A customer must always know where their money is. Every journey below ends in a definite state, and every uncertain state has an owner and a recovery path.

**Version 1.2 (21 Oct 2026).** Aligned with PRD 1.2, the approved UI and [Architecture Baseline 2.0](https://claude.ai/artifact/3TaqDBgNAXW9Y8P6cUW3xS). New journeys: payment requests from a conversation (§1A), team and approvals (§5), point of sale and refunds (§6), status posts (§7). Merchant onboarding now separates KYB approval from permission to collect. Status: draft for review; nothing here is built. 21 Oct 2026: consistency fix (§2 lead no longer implies KYB approval enables collection).

### State and label alignment (Open)

This document keeps the state names defined in 00, which governs. Baseline 2.0 and the approved UI use different names for some of the same states, and the word "reversal" currently has three meanings. Until the product owner decides (C1, C5, C6, plus the new item N13 below), read them as follows:

| 00 state (this document) | Baseline 2.0 state | UI label |
| --- | --- | --- |
| `rejected` (validation failed, no money moved) | refused at gate A; nothing created | "This business can't accept payments right now" or the limit message |
| `submitting`, `pending_customer` | `requested`, `processing` | Requested / Waiting for the customer |
| `uncertain` | `pending_confirmation` | Pending confirmation |
| `under_review` | `awaiting_resolution` | Awaiting resolution |
| `succeeded` | `completed` | Completed |
| `failed` | `failed` | Failed |
| (no 00 equivalent) | `voided`: closed before any provider attempt | Voided |

| Word | 00 meaning | Approved UI meaning | Baseline 2.0 meaning |
| --- | --- | --- | --- |
| Refund | Money returned to the customer through a provider | (the UI calls this a reversal) | Same as 00 |
| Reversal | Internal ledger correction, no money moves (00 §2.7) | Money the business asks MTN to send back | Money the provider pulls back and reports |

**N13 (new, Open): one vocabulary.** Product owner, with finance. Recommended: refund as in 00; *provider reversal* for money the provider pulls back; *ledger correction* for 00's internal reversal; then 00, 02, 03, 04 and Baseline 2.0 are amended together.

### Actors

| Actor | Role in flows |
| --- | --- |
| Customer | Pays, views receipts and history, saves contacts, messages businesses |
| Merchant owner | Onboards, completes KYC, receives payments, issues invoices, manages staff |
| Merchant staff | Acts within assigned permissions |
| Ops / support agent | Reviews KYC, investigates payments, resolves disputes |
| Scanpay backend | Validates, records state, posts ledger entries, emits events |
| Payment providers (MTN first; Airtel, Zamtel, card acquirer later) | Executes collections and disbursements; sends callbacks; answers status enquiries |
| Background workers | Reconciliation, status polling, notification delivery, expiry |

### Conventions

- States are written in `snake_case` and match the enums in 04.
- Each flow lists its PRD requirement IDs (01 §3).
- **Uncertain** means Scanpay does not yet know the provider outcome. Uncertain is never shown or treated as failed.
- Every client write carries an idempotency key; replays return the original result (05 §2).
- Assumptions about MTN behaviour are marked and must be checked against current MTN API documentation.

## 1. Customer pays a merchant (PAY-01 to PAY-17)

A payment moves through eight states; only a verified MTN outcome can make it `succeeded` or `failed`, and every timeout lands in `uncertain`, never `failed`.

### Happy path

1. Customer opens Scanpay and scans the merchant QR code (PAY-01).
2. Backend resolves the QR to a business and checks that it is currently eligible to collect, that no restriction blocks it, and that the amount is within its limits (PAY-22). KYB approval alone is not enough.
3. App shows the verified business name, logo and verification badge; the customer confirms the recipient (PAY-02).
4. Customer enters or confirms the amount and sees any fee and total (PAY-03).
5. Customer confirms; the app sends the request with a client-generated idempotency key. Payment is `created` (PAY-10).
6. Backend validates limits, amount, authorization and sandbox eligibility (PAY-11, REG-02). Invalid → `rejected` with a reason; no money moves.
7. Backend persists the provider reference, then calls the provider (MTN request-to-pay at G1); payment is `submitting`. On provider acknowledgement it becomes `pending_customer`.
8. Customer approves in the provider's own prompt (MTN PIN prompt at G1; Airtel, Zamtel and card authentication defined per adapter). Credentials are entered with the provider, never in Scanpay.
9. The provider sends a callback. Backend stores it, then confirms the outcome by status query with the same reference and records the evidence (T3, 00 §3) (PAY-12). The callback alone never moves money (00 §2.3).
10. From recorded evidence, in one database transaction (T4): payment → `succeeded`, balanced ledger entries posted, outbox event written (PAY-14).
11. Customer sees success and a receipt; merchant receives a notification (PAY-06, NOT-01).

&#91;embedded content: payment collection state machine · 8 states\]

Read left to right for the normal route; everything that drops into `uncertain` is resolved by status enquiry or, failing that, by an operations review.

### Failure and recovery

| Situation | Resulting state | What the customer sees | Recovery |
| --- | --- | --- | --- |
| Merchant inactive or unverified | `rejected` | “This business can’t accept payments right now” | None needed; no money moved |
| Amount over limit | `rejected` | Limit message and the allowed maximum | Customer adjusts amount |
| Network drop before request reaches Scanpay | none created | Retry prompt | Retry with the same idempotency key |
| Customer taps Pay twice | single payment | One payment in progress | Idempotency key returns the first result |
| MTN call times out | `uncertain` | “We’re confirming your payment” | Status enquiry job; never auto-fail |
| Customer declines or ignores MTN prompt | `failed` once MTN verifies | “Payment not completed” | Customer may start a new payment |
| No callback by deadline | `uncertain` | “We’re confirming your payment” | Status enquiry with backoff |
| Duplicate callback | unchanged | Nothing new | Ignored after first verified transition |
| Callback fails authentication | unchanged | Nothing new | Logged as security event; enquiry decides |
| Enquiry attempts exhausted | `under_review` | “Under review — we’ll update you” with case reference | Ops case opened (§4) |
| Customer reports missing payment | dispute case linked | Case reference and expected response time | Ops investigation (§4) |

Each row is an instance of the provider failure matrix in 00 §4 (F1–F15), which governs retry limits, stop rules and escalation owners.

Branches the table above does not show, all governed by 00 §4:

- **Worker crash after T1** (F13): the request may or may not have reached the provider. The payment becomes Confirming, and the provider's ambiguous-dispatch rule applies; a "not found" query never triggers a fresh request.
- **Provider accepted but the response was lost** (F2): resolved by query with the same reference.
- **Status query stale or unavailable** (F7): Confirming persists within stop rules, then under review.
- **Late success after an apparent failure** (F14): the payment stays as recorded, a case opens, and money is corrected only by refund or reversal with approval.
- **Duplicate provider reference** (F4/F14): treated as an integrity incident, never as a second payment.
- **Statement contradicts an earlier query** (F14): reconciliation case; no automatic change.

`failed` and `rejected` are used only when their evidence criteria in 00 §2.1 are met.

**Assumption (per provider):** each provider, starting with MTN, supports a status query by Scanpay's reference for collections, payouts and refunds, and documents its callback authentication; rows it cannot support get a documented compensating control, and its callback authentication method is documented. To be confirmed in 05 §3.

### 1A. Business requests payment from a conversation (PAY-20, PAY-21, CNV-01, CNV-02) — G1

This is the journey the first build slice proves end to end.

1. A business member with `payment.request` opens a customer conversation and taps Request payment, optionally from an issued invoice (for example SP-INV-2048: 850.00 total, 300.00 paid, 550.00 outstanding).
2. The app sends the amount, currency, customer, purpose and a client idempotency key. Backend checks the member's permission, the business's eligibility, restrictions and limits, and that no other request is open for that invoice; it reserves limit capacity in the same transaction. If any check fails, nothing is created and the member sees the reason (PayOff, PayLimit screens).
3. A payment card appears in the conversation for both sides, showing Requested.
4. The customer approves in MTN's prompt. The card moves to Completed only after the payment is verified (§1 steps 9–10); a receipt card follows, and the invoice balance updates.
5. If MTN does not answer, the card shows Pending confirmation, and the business sees "Don't request it again": Request payment is blocked for that invoice while the payment is unresolved.
6. If the business is paused, or becomes ineligible, after the request is created but before it reaches MTN, the request is voided; nothing was sent, so no money can have moved.

| Situation | Card shows | What happens |
| --- | --- | --- |
| Member taps Request twice | One card | Same idempotency key returns the first request |
| Second request for the same invoice while one is open | Refused | "A payment request is already open for this invoice" |
| MTN times out | Pending confirmation | Status enquiry; never shown as failed |
| Inquiry attempts exhausted | Awaiting resolution | Payment operations case (§4) |
| Verified amount differs from requested (1,200.00 asked, 1,000.00 confirmed) | Completed, 1,000.00 | Receipt, invoice and ledger all use the verified amount |
| Late duplicate confirmation | Unchanged | Ignored; recorded |

### Separate lifecycles

A collection, its ledger posting, the merchant payout and any reversal each have their own state and records (04 §3):

- **Collection** — payment intent states above (00 §2.1); each provider interaction is a separate provider attempt (00 §2.2).
- **Posting** — committed only in T4, keyed once per effect; never edited (00 §2.4).
- **Payout** — `scheduled` → `reserved` → `submitting` → `pending_provider` → `paid` / `failed` / `uncertain` → `under_review`; never `paid` without verified evidence (00 §2.5).
- **Refund** — money returned to the customer through a provider; own key, reference, evidence and states; cumulative refunds never exceed the original (00 §2.6).
- **Reversal** — internal ledger correction with no provider money movement; proposed, approved by a second person, then posted (00 §2.7).

The original payment stays `succeeded`; its net position is derived from linked refunds and reversals.

### Resilience on the device

| Event | Behaviour |
| --- | --- |
| App killed or restarted mid-payment | App resumes from the stored payment ID and fetches current state; never re-submits |
| Connectivity lost after tapping Pay | Retry uses the same idempotency key; the screen shows Confirming, not an error |
| Duplicate submission | Same key returns the existing payment |
| Confirmation delayed | Confirming screen persists; push or poll updates it; history shows the item pinned as unconfirmed |
| Offline history | Cached items are labelled with last-updated time and never shown as live confirmation |

### Sandbox-specific steps (GD-2, when BoZ specifies them)

- Eligibility check for pilot users and merchants before onboarding completes.
- Required disclosures and consent captured and recorded before first payment.
- Transaction and volume limits enforced at validation (`rejected` with a clear reason).
- Complaints and escalation path available from every payment detail and receipt.
- Exact wording and limits are inserted only once BoZ conditions are confirmed.

## 2. Onboarding, authentication and business KYC (USR, KYC)

Customers can pay within minutes of sign-up; merchants can receive payments only after KYB approval **and** a separate eligibility decision approved by a second staff member (KYC-05, ELG-01).

### Customer registration and sign-in

1. Customer enters their mobile number; Scanpay sends a one-time code (USR-01).
2. Customer verifies the code, sets a name and creates an app passcode or enables device biometrics.
3. Device is registered; a session is issued (USR-02).
4. Later sign-ins use passcode or biometrics on a registered device; a new device requires a one-time code and notifies the old device (USR-04).
5. Sensitive actions (changing number, adding a payout account, approving large payments) require step-up verification (USR-06).

**Open decision:** exact factors, code channel (SMS vs WhatsApp) and lockout rules (05 §4).

| Failure | Behaviour |
| --- | --- |
| Code not received | Resend after cooldown; alternative channel if configured |
| Too many wrong codes or passcodes | Temporary lockout; security notification |
| Lost phone | Recovery via verified number + support check; all sessions revoked |
| Session stolen or suspicious | User or ops revokes sessions; re-authentication required |

### Merchant onboarding and KYC

1. Owner registers as a customer (everyone registers fresh; no accounts carry over from the old system), then chooses “Create a business”. Business is created in `draft` (KYC-01).
2. Owner enters business details, representative details and the settlement account money is paid out to, with its name-match result. Whether Scanpay pays out at all depends on DEC-18, so this item is provisional.
3. Owner uploads required documents; files are stored encrypted with restricted access (KYC-02).
4. Owner submits → `submitted`. A compliance reviewer picks it up → `in_review`. Each document is a separate submission; a resubmitted document is added, and the earlier one stays on record.
5. Reviewer decides: `approved`, `rejected` (with reason) or `more_info_needed` (with a specific request) (KYC-04). The owner may withdraw a draft or a case waiting for more information. A decided case never reopens; a later review opens a new case.
6. KYB approval does not switch payments on. A compliance staff member then records the business as eligible to collect, and a second staff member approves it; only then is a QR code issued and collection enabled, within the business's limits (KYC-05, ELG-01). Collecting during review is off by default (DEC-34 Open).
7. Material changes (owner, payout destination, legal name) require owner step-up verification, return the business to review, and need independent approval by an ops reviewer who did not raise the change; payouts to a new destination are held until approved (KYC-06).

**Assumption:** the required document list and tiered limits come from Zambian requirements and MTN merchant terms; to be confirmed before G1.

### Staff invitation

1. Owner invites a member by phone number and assigns one of four roles: Owner, Finance, Cashier or Viewer (DEC-31 Open). Inviting someone as Owner or Finance needs another Owner's approval and step-up (§5).
2. The invitee receives an SMS link, signs in or registers, and accepts. Acceptance works only if their verified phone number is the invited one; a forwarded link fails. Invitations expire (proposed 7 days, N6 Open); resending cancels the old link.
3. Staff see only the business data and actions their role allows (USR-03, USR-05). Removing staff revokes access immediately.

## 3. Notifications, contacts, invoicing and messaging

These flows build on verified payments and identities; none of them can change a payment's state.

### Notifications (NOT-01 to NOT-05) — G1

1. A state change commits together with an outbox event in the same transaction.
2. A worker reads the outbox and creates notification jobs per recipient and channel.
3. Push is sent; lock-screen text is generic (“Payment received”), amounts and names appear only in-app (NOT-04).
4. Delivery result is recorded; failures retry with backoff, then fall back to in-app inbox.
5. The app always reads payment state from the API, never from the notification (NOT-03).

### Contacts (CON-01 to CON-04) — G2

1. After a successful payment, the customer may save the merchant as a contact.
2. Customer searches businesses by name, Scanpay ID or QR; results show verification badges (CON-03).
3. Customer–customer discovery by phone number is opt-in on both sides; no contact list is uploaded without explicit consent (CON-04).
4. Paying a saved contact still re-resolves and shows the verified identity before confirmation (PAY-02).

### Invoicing and payment requests (INV-01 to INV-06) — G3

1. Merchant creates an invoice in `draft`: customer, line items, taxes, due date (INV-01, INV-02).
2. Merchant issues it → `issued`; the customer gets a notification and an invoice card in their conversation. An issued invoice is never edited: a change is an adjustment, requested by any invoicing member and approved by an Owner (INV-06, DEC-25 Open).
3. Customer pays through the standard payment flow (§1). The payment carries the invoice ID.
4. Only a `succeeded` payment updates the invoice: `partially_paid` or `paid` (INV-04).
5. Past due date with a balance → the overdue flag is set; the state stays `issued` or `partially_paid`. Cancelling an issued invoice with nothing paid against it needs Owner approval.
6. Receipt and invoice PDF are available to both parties (INV-05).

A failed or uncertain payment never changes invoice status. ZRA Smart Invoice submission is not part of this flow until the separate ZRA workstream is approved (06 §3).

### Messaging and calling (MSG-01 to MSG-10) — G1 text and cards, G4 media

**Text and record cards (G1)**

1. A conversation opens from a contact, a payment receipt or an invoice; only members can read it (MSG-06).
2. Messages are sent with a client ID so retries don't duplicate; states: `sending` → `sent` → `delivered` → `read` (MSG-07).
3. Payment requests and receipts can be shared into a conversation as structured cards; tapping one opens the real record.
4. Users can block or report; blocked users cannot message or call (MSG-09).

**Voice notes (G4)**

1. User records; the app uploads the audio, then sends a message referencing it.
2. Upload failure keeps the note locally with a retry option; nothing is sent half-uploaded.

**Audio, video and conference calls (G4)**

1. Caller starts a call; permissions are requested at that moment (MSG-10).
2. Callee devices ring via push; states: `inviting` → `ringing` → `accepted` / `declined` / `missed` → `ended` or `failed` (MSG-08).
3. Media flows peer-to-peer or via a relay server when needed (05 §5).
4. On network loss, the app attempts reconnection for a short window before ending the call.
5. Conferences add a participant list with join/leave events; the host can remove participants (MSG-05).

## 4. Operations, support and background processes

Every uncertain or disputed payment becomes an owned case with an audit trail; no one edits a balance directly.

### Background jobs

| Job | Trigger | What it does | Never does |
| --- | --- | --- | --- |
| Status enquiry | Payment in `uncertain` or `pending_customer` past deadline | Queries the provider by the same reference within stop rules (00 §4); records evidence; applies via T4 | Mark failed without MTN confirmation |
| Escalation | Enquiry attempts exhausted | Moves payment to `under_review`, opens ops case | Close the case |
| Reconciliation | Daily, plus on demand | Matches internal records to each enabled provider's statements; records breaks as cases with amount, provenance and owner | Auto-correct balances |
| Payout run | Per merchant settlement schedule (open decision) | Creates payouts from available merchant balances | Mark `paid` before MTN confirms |
| Notification delivery | Outbox event | Sends and retries notifications | Change financial state |
| Invoice ageing | Daily | Sets the overdue flag on past-due invoices | Touch payments |
| Session and code expiry | Scheduled | Expires codes, idle sessions, stale drafts | Delete financial records |

### Payment investigation

1. Case is opened by escalation, a customer report (PAY-08) or a reconciliation break.
2. Agent sees the payment timeline: requests, MTN references, callbacks, enquiries, postings, notifications.
3. Agent attaches provider evidence obtained by the system (query result or statement line). Agents cannot type in or upload evidence that the system treats as provider-verified.
4. Agent proposes a resolution: apply verified evidence, issue a refund, or post a reversal (00 §2.6–2.7).
5. A second authorized person approves any resolution that posts or reverses money (LED-07).
6. System applies the transition and postings in one transaction; customer and merchant are notified; case closes with a reason.

### Reconciliation breaks

| Break | Meaning | Handling |
| --- | --- | --- |
| MTN success, Scanpay not succeeded | Missed or rejected callback | Apply verified outcome via investigation |
| Scanpay succeeded, not in MTN statement | Possible timing difference or serious defect | Hold; re-check next cycle; escalate if persists |
| Amount mismatch | Defect or data issue | Freeze affected merchant payouts; incident |
| Duplicate MTN reference | Duplicate processing attempt | Incident; confirm a single posting exists |

### Administrative access

The operations console has its own sign-in. Staff roles are payment operations, compliance, risk, support and platform admin (DEC-31 Open); each final payment state, eligibility decision, restriction and limit change needs a second staff member. Every view of KYC documents and every financial action is audit-logged with actor, reason and time (PAY-17).

## 5. Team, approvals and business controls (TEAM, APR, ELG) — G1

### Approval of a sensitive action

1. A member asks for an action that needs approval (for example a refund). The owning area records the request and sends it to approval; nothing changes yet.
2. Eligible approvers see it in Approvals on every device. The requester can see it in My requests and can withdraw it.
3. An approver who is not the requester, and not the person the action is about, approves or rejects, with step-up where the policy requires it.
4. On approval, the owning area rechecks everything (still permitted, nothing changed meanwhile, same details as approved, still within the time window) and only then acts. If a check fails, the action is refused and the requester is told why.
5. Approval never moves money: a refund shows "Approved · waiting for MTN" until MTN confirms.

| Situation | Outcome |
| --- | --- |
| Requester tries to approve their own request | Refused |
| No one other than the requester can approve (a business with one Owner) | Request can't be created (N1 Open) |
| Approval window passes | Expired; ask again |
| Approved but not acted on in time | Lapsed; ask again |
| Requester leaves the business | Waiting requests are withdrawn |

### Team changes

- Role changes, suspension, reactivation and removal are done by an Owner. Promotion to Owner or Finance needs a second Owner's approval and step-up.
- The last active Owner cannot leave, be suspended, removed or demoted; the screen says another Owner is needed first.
- A suspended or removed member's next action is refused at once.

### Eligibility, restrictions and limits (staff)

1. A staff member proposes a change with a reason in the operations console; a second staff member approves it.
2. The change can take effect now or at a set future time; a scheduled change can be cancelled, with approval, before it starts.
3. A collections pause stops new payment requests and voids requests not yet sent to MTN; payments already with MTN still complete or resolve; refunds, provider reversals and reconciliation continue.

## 6. Point of sale (POS) — G3, and refunds (RFD) — G1

### Point of sale

1. A cashier opens their till with a starting float (rules: DEC-37 Open).
2. They ring up a sale and choose cash or MTN. Cash completes the sale at once.
3. For MTN, a payment request is sent for the sale; the sale is held until the payment is verified. While it is unresolved the sale cannot be abandoned, cash cannot be taken for it, and the till cannot close.
4. At cash-up the cashier counts the drawer; the expected amount, the count and any difference are recorded, and the till closes.
5. A return with a refund goes through approval (§5).

### Refund

1. A member requests a refund of a completed payment, up to the amount not already refunded or being refunded.
2. An Owner approves with step-up (§5).
3. Payments rechecks, records the refund as authorized, then sends it to MTN.
4. The refund becomes refunded only when MTN confirms; a timeout shows Refund pending, never refunded or failed.
5. The invoice balance, the ledger and the payment card update only on the confirmed refund.

Whether MTN Zambia supports refunds through its API is still being confirmed (MTN questionnaire); until it is, this flow is blocked for MTN.

## 7. Status posts and Discover — G2

1. A business Owner, or a member the Owner has allowed, posts a status update and picks how long it stays up.
2. The business's contacts see it in Status; it ends at the chosen time or when the author removes it.
3. Anyone who sees a post can report it; reported posts go to a moderation queue in the operations console, where staff can take it down.
4. Discover ranking, boosting and moderation rules are DEC-36 Open; they are designed before this journey is built.

## 8. Open questions and cross-references

- [ ] For each provider (MTN first): callback authentication, status-query rate limits, refund support and statement format.
- [ ] What are the status-enquiry deadline and maximum attempts before `under_review`? (Set per provider from testing; recorded as stop rules in 00 §4.)
- [ ] Merchant settlement: instant payout per payment, or scheduled batch? Affects ledger accounts and payout flow.
- [ ] Refund policy: who can initiate, within what window, and who bears fees?
- [ ] Customer authentication factors and recovery checks.
- [ ] KYC document list and limits per tier.
- [ ] BoZ sandbox conditions: eligibility, limits, disclosures, consent, complaints and reporting.

* One vocabulary for states and for refund and reversal (N13, above).
* Who approves a refund in a business with a single Owner (N1).

**Cross-references:** requirements in 01 §3 · screens for each state in 03 §4 · state enums and tables in 04 §3 · idempotency, callbacks and enquiry design in 05 §2–3 · test cases for every row of the failure tables in 06 §5.
