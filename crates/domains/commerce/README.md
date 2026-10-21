# commerce (chunk 3)

**Status:** Not frozen as of 21 Oct 2026. No tables, migrations or domain logic until this chunk passes its freeze gate (06 §2A).

**Specification:** 04 §8.2

**Owns:** Customers and contacts, products and prices, invoices, adjustments and allocations.

**Blocking decisions:** DEC-25, 26, 17; C4; N9.

## Rules

- `src/domain/` holds pure logic (types, state machines, policy); no I/O.
- `src/db/` reads and writes only the `commerce` schema, as this domain's producer role.
- Other domains are used only through their commands, outbox events and read-only contract views. No cross-schema foreign keys.
