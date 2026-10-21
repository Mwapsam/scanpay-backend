# Scanpay 03 — Design Brief

Oct 18, 2026 · @TCS

## 1. Purpose, brand and voice

Scanpay's design must make one thing unmistakable on every screen: where the customer's money is right now. Trust and clarity come before decoration.

**Governed by 00.** Every state a screen shows comes from the 00 Product and Financial Integrity Contract; no design may imply a financial outcome the backend has not committed. Screens are provider-neutral: MTN Mobile Money first, with Airtel Money, Zamtel Kwacha and cards shown only once enabled (00 GD-1). Pilot screens carry the disclosures BoZ requires for the sandbox (00 GD-2). The app is designed from scratch as a new React Native (Expo) app for Android and iOS; nothing is inherited from the old app's screens or flows (00 GD-3, GD-4).

*Business summary.* The brand promise is **Pay. Connect. Grow.** The product should feel calm, local and dependable — a tool Zambian customers and small businesses can rely on at a busy till with a weak signal.

**Version 1.2 (21 Oct 2026).** The full UI now exists and is the reference for every screen: [Cross-platform UI/UX](https://claude.ai/artifact/1r2Uypv2kjvkZugFCGZXyk) (mobile, tablet, web workspace and operations console) with the [Master Screen Inventory](https://claude.ai/artifact/Ln1ovJpNYGsePLu4ARUqBd). This brief now sets the rules those screens follow; where a screen and this brief disagree, raise it rather than changing either silently.

Still to do before UI sign-off: the Android render pass and the accessibility pass (§5). Payment labels below follow the approved UI; the underlying state names are being reconciled with document 00 (N13, see 02, State and label alignment).

### Brand attributes

| Attribute | Means | Does not mean |
| --- | --- | --- |
| Trustworthy | Verified names, plain outcomes, receipts always available | Bank-like coldness or legal jargon |
| Clear | One primary action per screen; states named in words, not just colours | Minimalism that hides fees or status |
| Local | Zambian names, Kwacha formatting, MTN flows that match what users know | Stock imagery of other markets |
| Growing | Merchant tools that appear as businesses need them | Upselling during a payment |

### Voice and tone

- Plain English at a reading age most customers are comfortable with; short sentences.
- Say what happened and what happens next: “We’re confirming your payment with MTN. You don’t need to pay again.” (provider name varies by method)
- Never blame the user; never say “failed” when the outcome is unknown.
- Amounts always as **K 1,250.00** (ZMW), with the currency shown.
- Merchant-facing copy may be more compact; operations copy may be technical.

**Open decision:** local-language support (e.g. Bemba, Nyanja) for launch or later (§5).

## 2. Visual direction

The starting palette and type are carried over from the Scanpay executive pitch deck; they are a **proposal** for the product until tested for contrast, outdoor legibility and low-end Android rendering.

### Colour tokens (proposed)

| Token | Hex | Use |
| --- | --- | --- |
| `brand.deep` | #0F2A2E | Primary dark surfaces, headers, merchant dashboard chrome |
| `brand.ink` | #14262A | Primary text on light surfaces |
| `brand.paper` | #F6F4EE | Primary light background |
| `brand.sand` | #ECE8DD | Cards and secondary surfaces |
| `brand.sage` | #BFD3CF | Secondary text on dark, dividers |
| `accent.ember` | #A84E16 | Primary action on light backgrounds |
| `accent.ember-light` | #E8954F | Primary action and highlights on dark backgrounds |
| `status.success` | to define | Verified success only |
| `status.pending` | to define | Pending and uncertain (calm, not alarming) |
| `status.error` | to define | Verified failure, validation errors |

The three status tokens are defined and contrast-tested before the component library is built (deliverable in §6). Status colours must never be the only signal: every status also has an icon and a word. All text/background pairs must meet WCAG 2.2 AA (4.5:1 body, 3:1 large text and UI); `accent.ember-light` on `brand.paper` likely fails and must not be used for text there.

### Typography (proposed)

- **Space Grotesk** — headings, amounts, merchant names.
- **DM Sans** — body, labels, buttons.
- Amount on the confirm and result screens is the largest element on the page; tabular figures for all money.
- Minimum body size 16 px equivalent; support system font scaling to 200% without truncating amounts or merchant names.

### Iconography and imagery

- One outlined icon set; status icons are distinct in shape, not just colour.
- Merchant logos shown only after verification; unverified businesses get a neutral initial avatar.
- Illustrations, if used, feature Zambian settings and people; no stock imagery implying a user base Scanpay doesn't have.

## 3. UX principles and payment-status design

The payment screens are the product's most important design work; every state in 02 §1 has a designed screen, and no screen invents a state the backend doesn't have.

### Principles

1. **Confirm the recipient before the amount is final.** Verified business name, badge and logo are shown above the amount (PAY-02).
2. **One primary action per screen.** Secondary actions are visually quieter.
3. **Never imply failure without proof.** Uncertain gets its own calm treatment and copy.
4. **Make retries safe and obvious.** Double taps are absorbed; “Pay again” appears only after a verified failure.
5. **Receipts are permanent.** Every finished payment has a receipt reachable from history and the result screen.
6. **Mobile-first, one-handed.** Primary actions in thumb reach; works on small Android screens.
7. **Progressive disclosure for merchants.** New tools appear as the business needs them, never mid-payment.

### Payment status presentation

| Backend state (04) | Label shown | Icon / colour | Primary copy | Primary action |
| --- | --- | --- | --- | --- |
| `created`, `submitting` | Sending | Spinner / neutral | “Sending your payment request…” | none (no cancel once submitted) |
| `pending_customer` | Approve on your phone | Phone / pending | “Check your phone and approve K 1,250.00 to Mama’s Kitchen in the {provider} prompt.” | none; timer hint |
| `uncertain` | Pending confirmation | Clock / pending | “We’re confirming this payment with {provider}. Don’t pay again — we’ll notify you.” | View details |
| `under_review` | Awaiting resolution (C5 Open) | Magnifier / pending | “We’re checking this payment. Case SP-… — we’ll update you.” | Contact support |
| `succeeded` | Paid | Check / success | Amount, merchant, time, reference | View receipt; Save contact |
| `failed` | Not completed | Cross / error | Plain reason if known; “No money left your account.” only when verified provider evidence specifically supports it; otherwise say only what is known | Try again |
| `rejected` | Couldn’t start | Info / error | Specific reason (limit, merchant unavailable) | Change amount / Back |
| voided (Baseline 2.0; no 00 equivalent yet) | Voided | Dash / neutral | “This request was closed before it was sent. No money moved.” | Request again |
| Business view of a request in a conversation | Same labels on the payment card | As above | While pending: “Don’t request it again — we’re confirming with MTN.” | Request payment disabled for that invoice until resolved |

*Example merchant name is illustrative only.*

`succeeded` is shown as Paid only after the backend has committed the posting (00 §3, T4). Recorded but unapplied evidence still displays as Confirming.

### Other lifecycles shown to users

| Lifecycle (00 §2) | States shown | Label rules |
| --- | --- | --- |
| Payout (merchant) | Scheduled · Sending · Paid out · Not completed · Confirming · Under review | “Paid out” only after verified provider evidence; collections and payouts are shown on separate tabs so a sale is never read as settled |
| Refund (customer and merchant) | Requested · Waiting for approval · Approved · waiting for MTN · Refund pending · Refunded · Not completed · Awaiting resolution | Shown as its own item linked to the original payment, which stays Paid with a “Refunded K x” note |
| Reversal | Not shown as a customer action | Meaning not settled (N13): the approved UI uses “reversal” for what 00 calls a refund. Until decided, the business-initiated flow is designed as a refund, and money the provider pulls back appears as a labelled provider reversal on the payment |
| Invoice | Draft · Issued · Part paid · Paid · Cancelled, with an Overdue flag on issued and part-paid invoices | Changes only from committed payments; an invoice PDF never says “paid” unless the allocation is committed |

### Trust patterns

- Verification badge with a tap-to-explain sheet (“Scanpay checked this business’s registration”).
- Unverified or suspended merchants: payment is blocked with a clear message, never just a warning.
- Provider credentials are entered only in the provider’s own prompt (mobile-money PIN) or the card processor’s secure fields; Scanpay never shows a field for them. App passcodes and biometrics only unlock Scanpay; biometric data never leaves the device.
- Lock-screen notifications are generic; details shown after unlock (NOT-04).
- Security events (new device, number change) shown in an account activity list.

## 4. Key screens by domain and gate

All screen families below are now designed in the approved UI; the Master Screen Inventory is the complete list. Each gate's screens are retested against this brief before that gate's build starts.

| Gate | Domain | Screens | Notes |
| --- | --- | --- | --- |
| G1 | Onboarding | Welcome, phone entry, code verify, passcode/biometrics, new-device notice | Works offline-first until network needed |
| G1 | Payments (customer) | Home with Scan button, payment-method choice (enabled methods only), QR scanner with manual-entry fallback, recipient confirm, amount + fee, approve-on-phone, result (all states in §3), receipt | Scanner must handle glare and damaged codes; manual Scanpay ID fallback |
| G1 | History | Transaction list with status chips, filters, transaction detail with timeline, report a problem | Uncertain items pinned to top |
| G1 | Merchant | Business sign-up, KYC steps with progress, document upload, KYC status, QR display/print, incoming payments feed, payment detail, daily summary | Large-type “Payment received” confirmation for counter use |
| G1 | Account | Profile, security, sessions/devices, notification settings, help |  |
| G1 | Ops console | KYC queue and review, eligibility, restrictions and limits (two-person), payment operations and investigation, case view with timeline, approval queue, reconciliation, disputes, status moderation, audit log | Desktop web; dense but clear |
| G2 | Contacts | Contact list, business search, business profile, privacy settings | Verified badges throughout |
| G1 | Messaging (text) | Chat-first shell (Chats, Contacts, Discover, Payments, Me), conversation, request payment from chat, invoice / payment / receipt cards, block and report | Payment cards open real records |
| G3 | Invoicing | Invoice list, editor, preview, send, customer invoice view, pay invoice | Invoice status separate from payment status |
| G3 | Merchant tools | Reports, catalogue, point of sale, till and cash-up, returns (refund screens themselves are G1) |  |
| G4 | Calls | Voice-note recorder, incoming call, in-call (audio/video), conference participants, call history | Permission prompts in context |
| G1 | Business workspace (web and tablet) | Overview, inbox, transactions, investigation, invoices, ledger, reconciliation, approvals, team and roles, invitations, verification, reports, settings | Every action shown only to roles allowed it, and enforced on the server too |
| G1 | Team and approvals | Members and roles, invite, accept invite, approvals inbox, my requests, step-up prompt | “Approved” never looks like money moved |
| G1 | Collections status | Collections off, collections unavailable, limit reached | Says why and what the business can do |
| G2 | Status and Discover | Post status, choose duration, view status, report, Discover | Ranking and boosting rules wait for DEC-36 |

### Merchant counter mode (G1)

A merchant at a till needs to see confirmed payments at a glance: a full-screen live feed of `succeeded` payments with amount and payer first name, an audible cue, and no pending or uncertain items shown as “received”. The feed shows its connection state and last update time; when disconnected it says so plainly and never displays cached items as new.

## 5. Accessibility, low connectivity and localisation

### Accessibility (WCAG 2.2 AA target)

- Contrast meets AA for all text and interactive elements.
- Touch targets at least 48 × 48 dp.
- Every status conveyed by icon + text, never colour alone.
- Screen-reader labels for scanner, amounts (read as “one thousand two hundred fifty kwacha”) and status changes (live region announcements).
- Text scaling to 200% without loss of amount or merchant name.
- Motion reduced when the system setting requests it.
- React Native (Expo) components use platform accessibility props (labels, roles, live regions) and are tested with TalkBack and VoiceOver on low-end Android and current iOS devices.

### Low connectivity and low-end devices

- Assume intermittent 3G/4G and data-cost sensitivity: small app size, compressed images, no autoplay media.
- Optimistic UI is **forbidden** for money: a payment is never shown as paid until the API confirms `succeeded`.
- Offline: history and receipts already loaded remain viewable; new payments need connectivity and say so.
- Status screens poll or reconnect gracefully and survive app restarts (resume to the right state from the payment ID).

### Localisation

- English at launch (assumption). Copy kept in string files from day one for later languages.
- ZMW formatting with thousands separators; dates as 18 Oct 2026; Africa/Lusaka time.
- Phone numbers in +260 format with local display.

### Regulatory disclosures (GD-2)

- Reserved placements: onboarding (before account creation), payment confirmation, receipt, help and complaints.
- Placements are designed now; wording is inserted only when BoZ sandbox conditions are confirmed. No regulatory wording is invented in advance.
- Consent capture is an explicit action with a recorded timestamp, not a pre-ticked box.

## 6. Deliverables, open decisions and cross-references

### Design deliverables per gate

- [ ] Tested colour and type tokens (contrast report)
- [ ] Component library: buttons, inputs, amount field, status chip, receipt card, merchant identity header, timeline
- [ ] High-fidelity flows for every state in 02 (designed in the approved UI; to be rechecked after N13)
- [ ] Copy deck for every payment state and error
- [ ] Usability test with customers and merchants before G1 build freeze (findings recorded, not assumed)
- [ ] Ops console designs (done in the approved UI); Android render pass and accessibility pass (outstanding)
- [ ] G2–G4 flows, each before its gate's build starts
- [ ] Approval-step designs for each enabled provider (MTN at G1; others at their sub-gates)
- [ ] Sandbox disclosure and consent placements

### Open decisions

| Decision | Options | Needed by |
| --- | --- | --- |
| Mobile client | Decided: new app, built from scratch (GD-3) | Before G1 design freeze |
| Mobile framework | Decided: React Native with Expo (GD-4, 05 §1) | Before G1 build |
| Final brand palette for product | Pitch-deck palette or adjusted | Before component library |
| Local languages at launch | English only / English + others | Before G1 copy freeze |
| Payment labels and refund vs reversal wording (C1, C5, C6, N13) | UI labels as in §3; one vocabulary across 00, 02, 03, 04 and Baseline 2.0 | Before G1 copy freeze |
| Status expiry options, Discover ranking, boosting (DEC-36) | Merchant chooses duration (decided 21 Oct); the rest open | Before G2 design freeze |

**Cross-references:** requirements 01 §3 · states and journeys 02 §1–4 · state enums 04 §3 · client architecture 05 §1 · design milestones 06 §3.
