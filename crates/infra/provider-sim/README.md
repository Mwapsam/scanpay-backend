# provider-sim

Scripted fake payment provider used to prove F1–F15 (00 §4) deterministically at G0 (06 §5).

Infra crates talk to external services only. They hold no business rules and own no tables; only the backend holds provider credentials, which come from the secret store and are never logged.
