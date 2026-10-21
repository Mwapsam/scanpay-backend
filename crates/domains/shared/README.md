# shared (chunk 1)

**Status:** Not frozen as of 21 Oct 2026. No tables, migrations or domain logic until this chunk passes its freeze gate (06 §2A).

**Specification:** Shared draft 0.7 (Claude Docs)

**Owns:** Users, credentials, devices, sessions, organizations, documents, audit log, outbox and jobs.

**Blocking decisions:** 0.7 freeze checklist; CR-1 to CR-11.

## Rules

- `src/domain/` holds pure logic (types, state machines, policy); no I/O.
- `src/db/` reads and writes only the `shared` schema, as this domain's producer role.
- Other domains are used only through their commands, outbox events and read-only contract views. No cross-schema foreign keys.
