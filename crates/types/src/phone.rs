use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::ValidationError;

const ZAMBIA_PREFIX: &str = "+260";
/// Digits after +260 in a Zambian number.
const ZAMBIA_SUBSCRIBER_DIGITS: usize = 9;

/// A phone number in E.164 form, for example `+260971234567`.
///
/// Personal data: `Debug` masks all but the last four digits and there is no
/// `Display`, so the full number only leaves through [`PhoneE164::as_str`].
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PhoneE164(String);

impl PhoneE164 {
    /// Accepts E.164 (`+` and 8–15 digits) or the Zambian local form
    /// `0XXXXXXXXX`, which is normalised to `+260XXXXXXXXX`. A single space
    /// or hyphen between two digits is ignored; any other separator, or one
    /// anywhere else, is rejected. A `+260` number must have exactly nine
    /// digits after the country code, and the first of them cannot be 0.
    pub fn parse(input: &str) -> Result<Self, ValidationError> {
        let chars: Vec<char> = input.trim().chars().collect();
        let mut compact = String::with_capacity(chars.len());
        for (i, &c) in chars.iter().enumerate() {
            if c == ' ' || c == '-' {
                let between_digits = i > 0
                    && chars[i - 1].is_ascii_digit()
                    && chars.get(i + 1).is_some_and(char::is_ascii_digit);
                if !between_digits {
                    return Err(ValidationError::InvalidPhone);
                }
            } else {
                compact.push(c);
            }
        }

        let normalised = if let Some(digits) = compact.strip_prefix('+') {
            if !(8..=15).contains(&digits.len()) || digits.starts_with('0') {
                return Err(ValidationError::InvalidPhone);
            }
            compact.clone()
        } else if let Some(rest) = compact.strip_prefix('0') {
            if rest.len() != ZAMBIA_SUBSCRIBER_DIGITS {
                return Err(ValidationError::InvalidPhone);
            }
            format!("{ZAMBIA_PREFIX}{rest}")
        } else {
            return Err(ValidationError::InvalidPhone);
        };

        if !normalised[1..].chars().all(|c| c.is_ascii_digit()) {
            return Err(ValidationError::InvalidPhone);
        }
        if let Some(subscriber) = normalised.strip_prefix(ZAMBIA_PREFIX)
            && (subscriber.len() != ZAMBIA_SUBSCRIBER_DIGITS || subscriber.starts_with('0'))
        {
            return Err(ValidationError::InvalidPhone);
        }
        Ok(Self(normalised))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_zambian(&self) -> bool {
        self.0.starts_with(ZAMBIA_PREFIX)
    }
}

impl FromStr for PhoneE164 {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for PhoneE164 {
    type Error = ValidationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl From<PhoneE164> for String {
    fn from(p: PhoneE164) -> Self {
        p.0
    }
}

impl fmt::Debug for PhoneE164 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let visible = self.0.len().saturating_sub(4);
        write!(f, "PhoneE164(+***{})", &self.0[visible..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_e164_and_zambian_local_forms() {
        for (input, expected) in [
            ("+260971234567", "+260971234567"),
            ("0971234567", "+260971234567"),
            ("+260 97 123 4567", "+260971234567"),
            ("097-123-4567", "+260971234567"),
            ("  +260971234567  ", "+260971234567"),
            ("+447911123456", "+447911123456"),
        ] {
            assert_eq!(PhoneE164::parse(input).unwrap().as_str(), expected, "{input:?}");
        }
    }

    #[test]
    fn rejects_malformed_numbers() {
        for bad in [
            "",
            "971234567",
            "260971234567",
            "+26097123456",
            "+2609712345678",
            "097123456",
            "+0971234567",
            "+1234567",
            "+1234567890123456",
            "+26097123456a",
            "+260+971234567",
            // subscriber number cannot start with 0
            "0012345678",
            "+260012345678",
            // separators only singly, between two digits
            "+ 260971234567",
            "+-2-6-0-9-7-1-2-3-4-5-6-7",
            "097  123 4567",
            "097--1234567",
            "-0971234567",
            "097.123.4567",
            "097\t1234567",
            "097\u{a0}1234567",
        ] {
            assert_eq!(PhoneE164::parse(bad), Err(ValidationError::InvalidPhone), "{bad:?}");
        }
    }

    #[test]
    fn zambian_check() {
        assert!(PhoneE164::parse("0971234567").unwrap().is_zambian());
        assert!(!PhoneE164::parse("+447911123456").unwrap().is_zambian());
    }

    #[test]
    fn debug_masks_the_number() {
        let phone = PhoneE164::parse("+260971234567").unwrap();
        let shown = format!("{phone:?}");
        assert_eq!(shown, "PhoneE164(+***4567)");
        assert!(!shown.contains("97123"));
    }

    #[test]
    fn serde_validates() {
        let phone: PhoneE164 = serde_json::from_str(r#""0971234567""#).unwrap();
        assert_eq!(serde_json::to_string(&phone).unwrap(), r#""+260971234567""#);
        assert!(serde_json::from_str::<PhoneE164>(r#""12345""#).is_err());
    }
}
