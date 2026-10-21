# sms

SMS delivery client (OTP, invitations). Vendor not chosen.

Infra crates talk to external services only. They hold no business rules and own no tables; only the backend holds provider credentials, which come from the secret store and are never logged.
