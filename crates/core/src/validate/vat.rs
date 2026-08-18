//! UK VAT registration number validation (HMRC modulus-97).
//!
//! Standard variant: weights 8..=2 over the first 7 digits of the 9-digit
//! body; check pair = 97 - (sum mod 97).
//! Alternative variant ("9755"): check pair = (97 - (sum mod 97)) + 55,
//! only valid when that value fits in two digits (<= 99).
//! A number is accepted if EITHER variant matches the final two digits.

const WEIGHTS: [u32; 7] = [8, 7, 6, 5, 4, 3, 2];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VatError {
    #[error("VAT body must be exactly 9 digits after stripping the GB prefix")]
    WrongDigitCount,
    #[error("modulus-97 checksum failed (standard and alternative variants)")]
    InvalidChecksum,
}

/// Validate a 9-digit VAT body against both checksum variants.
pub fn checksum_ok(digits: &[u8; 9]) -> bool {
    let d: Vec<u32> = digits.iter().map(|b| (b - b'0') as u32).collect();
    let sum: u32 = WEIGHTS.iter().zip(d[..7].iter()).map(|(w, x)| w * x).sum();
    let check_pair = d[7] * 10 + d[8];
    let std_check = 97 - (sum % 97);
    if check_pair == std_check {
        return true;
    }
    let alt_check = std_check + 55;
    alt_check <= 99 && check_pair == alt_check
}

/// Strip an optional "GB" prefix (case-insensitive), require exactly 9 ASCII
/// digits, validate the checksum, and return the canonical 9-digit body.
pub fn canonicalise_vat(raw: &str) -> Result<String, VatError> {
    let body = raw
        .strip_prefix("GB")
        .or_else(|| raw.strip_prefix("gb"))
        .unwrap_or(raw)
        .trim();
    if body.len() != 9 || !body.bytes().all(|b| b.is_ascii_digit()) {
        return Err(VatError::WrongDigitCount);
    }
    let mut digits = [0u8; 9];
    digits.copy_from_slice(body.as_bytes());
    if !checksum_ok(&digits) {
        return Err(VatError::InvalidChecksum);
    }
    Ok(body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_standard_variant() {
        // digits 1,2,3,4,5,6,7 -> weighted sum 8+14+18+20+20+18+14 = 112;
        // 112 mod 97 = 15; std check = 97-15 = 82 -> body "123456782".
        assert_eq!(canonicalise_vat("GB123456782").unwrap(), "123456782");
        // without prefix
        assert_eq!(canonicalise_vat("123456782").unwrap(), "123456782");
    }

    #[test]
    fn accepts_alternative_variant() {
        // digits 8x7 -> sum = 8*(8+7+6+5+4+3+2) = 280; 280 mod 97 = 86;
        // std = 11, alt = 11+55 = 66 (<= 99, valid) -> body "888888866".
        let body = "888888866";
        assert_eq!(canonicalise_vat(body).unwrap(), body);
    }

    #[test]
    fn rejects_wrong_digit_count() {
        assert_eq!(canonicalise_vat("GB12345678").unwrap_err(), VatError::WrongDigitCount); // 8 digits
        assert_eq!(canonicalise_vat("GB1234567890").unwrap_err(), VatError::WrongDigitCount); // 10 digits
        assert_eq!(canonicalise_vat("GB12345A789").unwrap_err(), VatError::WrongDigitCount); // non-digit
    }

    #[test]
    fn rejects_invalid_checksum() {
        // valid length, fails both variants (flip the check pair of a known-good)
        assert_eq!(canonicalise_vat("GB123456783").unwrap_err(), VatError::InvalidChecksum);
    }
}
