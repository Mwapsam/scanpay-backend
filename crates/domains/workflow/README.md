# workflow (chunk 4)

**Status:** Not frozen as of 21 Oct 2026. No tables, migrations or domain logic until this chunk passes its freeze gate (06 §2A).

**Specification:** Workflow Draft 0.3 (Claude Docs)

**Owns:** Approval requests, decisions and their execution contract.

**Blocking decisions:** DEC-23, 29/30, 31; N1, N3.

## Rules

- `src/domain/` holds pure logic (types, state machines, policy); no I/O.
- `src/db/` reads and writes only the `workflow` schema, as this domain's producer role.
- Other domains are used only through their commands, outbox events and read-only contract views. No cross-schema foreign keys.
