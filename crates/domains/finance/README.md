# finance (chunk 7)

**Status:** Not frozen as of 21 Oct 2026. No tables, migrations or domain logic until this chunk passes its freeze gate (06 §2A).

**Specification:** 04 §3 and §8.3

**Owns:** Ledger (accounts, journal entries, postings, balances), statement imports, matching, reconciliation cases.

**Blocking decisions:** DEC-01, 18, 17; C3.

## Rules

- `src/domain/` holds pure logic (types, state machines, policy); no I/O.
- `src/db/` reads and writes only the `finance` schema, as this domain's producer role.
- Other domains are used only through their commands, outbox events and read-only contract views. No cross-schema foreign keys.
