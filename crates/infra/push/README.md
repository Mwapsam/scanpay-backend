# push

Push notification client. Lock-screen text never carries amounts or names (NOT-04).

Infra crates talk to external services only. They hold no business rules and own no tables; only the backend holds provider credentials, which come from the secret store and are never logged.
