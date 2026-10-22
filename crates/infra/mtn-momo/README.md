# mtn-momo

MTN Mobile Money adapter implementing the payments provider trait. Empty until MTN Zambia's contract is confirmed in writing (docs/providers/); do not build from generic MoMo documentation.

Phone numbers: Scanpay's internal form is `scanpay_types::PhoneE164` (`+260…`). The form MTN expects on the wire (for example with or without the `+`) is converted here, and only from what MTN's written contract states.

Infra crates talk to external services only. They hold no business rules and own no tables; only the backend holds provider credentials, which come from the secret store and are never logged.
