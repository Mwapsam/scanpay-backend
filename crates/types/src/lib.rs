//! Validated value types used across Scanpay.
//!
//! Each type is parsed once, at the edge; holding a value of the type means it
//! is valid. Errors carry a stable [`ValidationError::code`] for the API error
//! shape (05 §2) and never echo the rejected input, which may be personal data.

mod error;
mod id;
mod money;
mod phone;

pub use error::ValidationError;
pub use id::{IdempotencyKey, OrganizationId, new_id};
pub use money::{Currency, Money};
pub use phone::PhoneE164;
