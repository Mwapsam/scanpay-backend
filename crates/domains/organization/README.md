# organization (chunk 2)

**Status:** Not frozen as of 21 Oct 2026. No tables, migrations or domain logic until this chunk passes its freeze gate (06 §2A).

**Specification:** Organization Draft 0.4 (Claude Docs)

**Owns:** Memberships and roles, invitations, KYB cases, collection eligibility, restrictions, limit definitions, settlement accounts.

**Blocking decisions:** DEC-31, 34, 35, 23, 28–30; N1, N2, N5, N6, N7, N12; KYB evidence list.

## Rules

- `src/domain/` holds pure logic (types, state machines, policy); no I/O.
- `src/db/` reads and writes only the `organization` schema, as this domain's producer role.
- Other domains are used only through their commands, outbox events and read-only contract views. No cross-schema foreign keys.
