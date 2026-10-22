/// Why a value was rejected. Messages never include the rejected input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("unsupported currency; only ZMW is accepted")]
    UnsupportedCurrency,
    #[error("amounts are in different currencies")]
    CurrencyMismatch,
    #[error("amount is out of range")]
    AmountOutOfRange,
    #[error("phone number must be in E.164 format, for example +260971234567")]
    InvalidPhone,
    #[error("idempotency key must be a non-nil UUID")]
    InvalidIdempotencyKey,
    #[error("id must be a non-nil UUID")]
    InvalidId,
}

impl ValidationError {
    /// Stable machine-readable code. Clients may rely on it; never change one.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedCurrency => "unsupported_currency",
            Self::CurrencyMismatch => "currency_mismatch",
            Self::AmountOutOfRange => "amount_out_of_range",
            Self::InvalidPhone => "invalid_phone",
            Self::InvalidIdempotencyKey => "invalid_idempotency_key",
            Self::InvalidId => "invalid_id",
        }
    }
}
