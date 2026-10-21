# payments (chunk 6)

**Status:** Not frozen as of 21 Oct 2026. No tables, migrations or domain logic until this chunk passes its freeze gate (06 §2A).

**Specification:** 04 §3 and §8.1

**Owns:** Payment intents, provider attempts, evidence, escalations, refunds, provider reversals, receipts, QR codes, limit reservations.

**Blocking decisions:** C1, C2, C5, C6, N13; DEC-27, 34, 35; N4, N8, N10, N11.

## Rules

- `src/domain/` holds pure logic (types, state machines, policy); no I/O.
- `src/db/` reads and writes only the `payments` schema, as this domain's producer role.
- Other domains are used only through their commands, outbox events and read-only contract views. No cross-schema foreign keys.
