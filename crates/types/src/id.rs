use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ValidationError;

/// A new time-ordered primary key (UUIDv7), generated in Rust (04 §1).
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}

/// Parses only the canonical text form: lower-case, hyphenated, 36
/// characters, no surrounding whitespace, not nil. `Uuid::parse_str` alone
/// would also accept braced, `urn:uuid:`, simple and upper-case spellings of
/// the same value, so two different raw strings could name one key.
fn parse_canonical(s: &str) -> Option<Uuid> {
    let id = Uuid::try_parse(s).ok()?;
    let canonical = id.hyphenated().to_string() == s;
    (canonical && !id.is_nil()).then_some(id)
}

/// Tenant id. On the server it comes from the authenticated membership, never
/// from a request body (05 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct OrganizationId(Uuid);

impl OrganizationId {
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl TryFrom<Uuid> for OrganizationId {
    type Error = ValidationError;

    fn try_from(id: Uuid) -> Result<Self, Self::Error> {
        if id.is_nil() {
            return Err(ValidationError::InvalidId);
        }
        Ok(Self(id))
    }
}

impl From<OrganizationId> for Uuid {
    fn from(id: OrganizationId) -> Self {
        id.0
    }
}

impl TryFrom<String> for OrganizationId {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<OrganizationId> for String {
    fn from(id: OrganizationId) -> Self {
        id.0.to_string()
    }
}

impl FromStr for OrganizationId {
    type Err = ValidationError;

    /// Canonical lower-case hyphenated form only (see `parse_canonical`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_canonical(s).map(Self).ok_or(ValidationError::InvalidId)
    }
}

impl fmt::Display for OrganizationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Client-generated `Idempotency-Key`: a non-nil UUID (05 §2) in its
/// canonical lower-case hyphenated form, so one key has exactly one spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(Uuid);

impl IdempotencyKey {
    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl FromStr for IdempotencyKey {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_canonical(s)
            .map(Self)
            .ok_or(ValidationError::InvalidIdempotencyKey)
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<IdempotencyKey> for String {
    fn from(k: IdempotencyKey) -> Self {
        k.0.to_string()
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_v7_and_ordered() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.get_version_num(), 7);
        assert!(a < b);
    }

    #[test]
    fn idempotency_key_accepts_uuids_only() {
        let key: IdempotencyKey = "0192d3a4-5b6c-7d8e-9f01-23456789abcd".parse().unwrap();
        assert_eq!(key.to_string(), "0192d3a4-5b6c-7d8e-9f01-23456789abcd");
        for bad in [
            "",
            "abc",
            "00000000-0000-0000-0000-000000000000",
            "0192d3a4",
            // other spellings of a valid key are refused
            "0192D3A4-5B6C-7D8E-9F01-23456789ABCD",
            "{0192d3a4-5b6c-7d8e-9f01-23456789abcd}",
            "urn:uuid:0192d3a4-5b6c-7d8e-9f01-23456789abcd",
            "0192d3a45b6c7d8e9f0123456789abcd",
            " 0192d3a4-5b6c-7d8e-9f01-23456789abcd",
        ] {
            assert_eq!(
                bad.parse::<IdempotencyKey>(),
                Err(ValidationError::InvalidIdempotencyKey),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn organization_id_rejects_nil() {
        assert_eq!(
            OrganizationId::try_from(Uuid::nil()),
            Err(ValidationError::InvalidId)
        );
        assert!(serde_json::from_str::<OrganizationId>(r#""00000000-0000-0000-0000-000000000000""#).is_err());
        let id = new_id();
        let org = OrganizationId::try_from(id).unwrap();
        assert_eq!(serde_json::to_string(&org).unwrap(), format!("\"{id}\""));
    }

    #[test]
    fn organization_id_text_matches_idempotency_key_rules() {
        let text = "0192d3a4-5b6c-7d8e-9f01-23456789abcd";
        assert!(text.parse::<OrganizationId>().is_ok());
        for bad in [
            " 0192d3a4-5b6c-7d8e-9f01-23456789abcd",
            "0192D3A4-5B6C-7D8E-9F01-23456789ABCD",
            "{0192d3a4-5b6c-7d8e-9f01-23456789abcd}",
        ] {
            assert_eq!(bad.parse::<OrganizationId>(), Err(ValidationError::InvalidId), "{bad:?}");
        }
        assert!(
            serde_json::from_str::<OrganizationId>(r#""{0192d3a4-5b6c-7d8e-9f01-23456789abcd}""#)
                .is_err()
        );
    }
}
