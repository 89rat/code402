pub mod distill;
pub mod identifiers;
pub mod vat;

/// UK Companies House company number format check.
/// Valid forms: exactly 8 alphanumeric characters that are either
/// all digits ("12345678") or a letter prefix (1-2 ASCII letters)
/// followed by digits ("SC123456", "NI123456", "OC334455").
pub fn validate_company_number_format(raw: &str) -> bool {
    let b = raw.as_bytes();
    if b.len() != 8 || !b.iter().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    if b.iter().all(|c| c.is_ascii_digit()) {
        return true;
    }
    let letters = b.iter().take_while(|c| c.is_ascii_alphabetic()).count();
    (1..=2).contains(&letters) && b[letters..].iter().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_all_digit_and_letter_prefixed() {
        assert!(validate_company_number_format("12345678"));
        assert!(validate_company_number_format("SC123456"));
        assert!(validate_company_number_format("OC334455"));
    }

    #[test]
    fn rejects_bad_shapes() {
        assert!(!validate_company_number_format("1234567"));   // too short
        assert!(!validate_company_number_format("123456789")); // too long
        assert!(!validate_company_number_format("ABC12345"));  // 3-letter prefix
        assert!(!validate_company_number_format("12SC3456"));  // letters not a prefix
        assert!(!validate_company_number_format("SC12345!"));  // non-alphanumeric
    }
}
