# mtn-momo

MTN Mobile Money adapter implementing the payments provider trait. Empty until MTN Zambia's contract is confirmed in writing (docs/providers/); do not build from generic MoMo documentation.

Infra crates talk to external services only. They hold no business rules and own no tables; only the backend holds provider credentials, which come from the secret store and are never logged.
