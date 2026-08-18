//! International identifier validation — deterministic checksums and formats.
//!
//! All algorithms are public ISO/industry standards. Test vectors are either
//! widely published known-good values or computed from the algorithm itself
//! (project convention: never copied from unverifiable external lists).

/// Base-36 character value: '0'-'9' -> 0-9, 'A'-'Z' -> 10-35 (uppercase input).
fn b36(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'A'..=b'Z' => Some((c - b'A') as u32 + 10),
        _ => None,
    }
}

/// ISO 13616 / ISO 17442 shared core: move `rotate` leading chars to the end,
/// expand letters to digits (A=10..Z=35), big-number mod 97 must equal 1.
fn mod97_10_ok(s: &[u8], rotate: usize) -> bool {
    let mut rem: u32 = 0;
    let mut process = |c: u8| -> bool {
        match b36(c) {
            Some(v) if v < 10 => rem = (rem * 10 + v) % 97,
            Some(v) => rem = ((rem * 10 + v / 10) % 97 * 10 + v % 10) % 97,
            None => return false,
        }
        true
    };
    for &c in &s[rotate..] {
        if !process(c) {
            return false;
        }
    }
    for &c in &s[..rotate] {
        if !process(c) {
            return false;
        }
    }
    rem == 1
}

// ---------- IBAN (ISO 13616) ----------

pub fn iban_ok(raw: &str) -> bool {
    let s: Vec<u8> = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    if s.len() < 15 || s.len() > 34 {
        return false;
    }
    if !s[0].is_ascii_uppercase() || !s[1].is_ascii_uppercase() {
        return false;
    }
    if !s[2].is_ascii_digit() || !s[3].is_ascii_digit() {
        return false;
    }
    if !s.iter().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    mod97_10_ok(&s, 4)
}

// ---------- LEI (ISO 17442) ----------

pub fn lei_ok(raw: &str) -> bool {
    let s: Vec<u8> = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    if s.len() != 20 || !s.iter().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    mod97_10_ok(&s, 0)
}

// ---------- ISIN (ISO 6166) — Luhn over letter-expanded digits ----------

pub fn isin_ok(raw: &str) -> bool {
    let s: Vec<u8> = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    if s.len() != 12 {
        return false;
    }
    if !s[0].is_ascii_uppercase() || !s[1].is_ascii_uppercase() {
        return false;
    }
    if !s[11].is_ascii_digit() {
        return false;
    }
    if !s[2..11].iter().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    // expand to decimal digit string
    let mut digits: Vec<u32> = Vec::with_capacity(24);
    for &c in &s {
        match b36(c) {
            Some(v) if v < 10 => digits.push(v),
            Some(v) => {
                digits.push(v / 10);
                digits.push(v % 10);
            }
            None => return false,
        }
    }
    // Luhn: double every second digit from the right, starting with the
    // digit immediately left of the check digit when the expanded length
    // is even; positions counted from the left (1-indexed): double odd
    // positions if total length is even, even positions if odd.
    let n = digits.len();
    let double_odd_positions = n % 2 == 0;
    let sum: u32 = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| {
            let pos1 = i + 1;
            let dbl = (pos1 % 2 == 1) == double_odd_positions;
            let v = if dbl { d * 2 } else { d };
            if v > 9 {
                v - 9
            } else {
                v
            }
        })
        .sum();
    sum % 10 == 0
}

// ---------- Generic Luhn (cards, IMEI, etc.) ----------

pub fn luhn_ok(raw: &str) -> bool {
    let s: Vec<u8> = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'-')
        .collect();
    if s.len() < 8 || s.len() > 19 || !s.iter().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let sum: u32 = s
        .iter()
        .rev()
        .enumerate()
        .map(|(i, b)| {
            let d = (b - b'0') as u32;
            let v = if i % 2 == 1 { d * 2 } else { d };
            if v > 9 {
                v - 9
            } else {
                v
            }
        })
        .sum();
    sum % 10 == 0
}

// ---------- SWIFT BIC (ISO 9362 format) ----------

pub fn bic_ok(raw: &str) -> bool {
    let s: Vec<u8> = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    if s.len() != 8 && s.len() != 11 {
        return false;
    }
    s[0..4].iter().all(|b| b.is_ascii_uppercase())
        && s[4..6].iter().all(|b| b.is_ascii_uppercase())
        && s[6..].iter().all(|b| b.is_ascii_alphanumeric())
}

// ---------- EAN-13 / UPC-A (GS1) ----------

pub fn ean13_ok(raw: &str) -> bool {
    let s: Vec<u8> = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'-')
        .collect();
    if s.len() != 13 || !s.iter().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let sum: u32 = s[..12]
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let d = (b - b'0') as u32;
            if i % 2 == 0 {
                d
            } else {
                d * 3
            }
        })
        .sum();
    let check = (10 - (sum % 10)) % 10;
    check == (s[12] - b'0') as u32
}

// ---------- GSTIN (India, GSTN published checksum) ----------

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GstinError {
    #[error("GSTIN must be exactly 15 alphanumeric characters")]
    WrongShape,
    #[error("invalid state code (must be 01-38)")]
    BadStateCode,
    #[error("characters 3-12 must match the PAN pattern (5 letters, 4 digits, 1 letter)")]
    BadPanPattern,
    #[error("character 14 must be 'Z'")]
    BadEntityMarker,
    #[error("checksum mismatch")]
    InvalidChecksum,
}

pub fn canonicalise_gstin(raw: &str) -> Result<String, GstinError> {
    let s: Vec<u8> = raw
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    if s.len() != 15 || !s.iter().all(|b| b.is_ascii_alphanumeric()) {
        return Err(GstinError::WrongShape);
    }
    let state = (s[0] - b'0') as u32 * 10 + (s[1] - b'0') as u32;
    if !(1..=38).contains(&state) || !s[0].is_ascii_digit() || !s[1].is_ascii_digit() {
        return Err(GstinError::BadStateCode);
    }
    // PAN pattern: chars 3-7 letters, 8-11 digits, 12 letter (1-indexed 3..12)
    if !s[2..7].iter().all(|b| b.is_ascii_uppercase())
        || !s[7..11].iter().all(|b| b.is_ascii_digit())
        || !s[11].is_ascii_uppercase()
    {
        return Err(GstinError::BadPanPattern);
    }
    if s[13] != b'Z' {
        return Err(GstinError::BadEntityMarker);
    }
    // GSTN checksum: base-36 values of first 14 chars, alternating factors
    // 1,2 starting with 1 at the leftmost char; sum quotients+remainders
    // mod 36; check = (36 - sum mod 36) mod 36.
    let mut sum: u32 = 0;
    for (i, &c) in s[..14].iter().enumerate() {
        let v = b36(c).ok_or(GstinError::WrongShape)?;
        let f = if i % 2 == 0 { 1 } else { 2 };
        let p = v * f;
        sum += p / 36 + p % 36;
    }
    let check = (36 - (sum % 36)) % 36;
    let expected = std::char::from_digit(check, 36)
        .unwrap()
        .to_ascii_uppercase() as u8;
    if s[14] != expected {
        return Err(GstinError::InvalidChecksum);
    }
    Ok(String::from_utf8(s).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iban_known_good() {
        // widely published test IBANs
        assert!(iban_ok("GB29 NWBK 6016 1331 9268 19"));
        assert!(iban_ok("DE89370400440532013000"));
    }

    #[test]
    fn iban_rejects() {
        assert!(!iban_ok("GB29NWBK60161331926818")); // flipped check digit
        assert!(!iban_ok("GB29NWBK")); // too short
        assert!(!iban_ok("1B29NWBK60161331926819")); // bad country letters
    }

    #[test]
    fn lei_self_consistent() {
        // vector computed from the algorithm (project convention)
        let mut body = b"549300AB12CD34EF56".to_vec(); // 18 chars
        // find check digits making mod97 == 1
        for a in b'0'..=b'9' {
            for b in b'0'..=b'9' {
                let mut s = body.clone();
                s.push(a);
                s.push(b);
                if mod97_10_ok(&s, 0) {
                    let lei = String::from_utf8(s).unwrap();
                    assert!(lei_ok(&lei));
                    let mut bad = lei.clone().into_bytes();
                    bad[19] = if bad[19] == b'0' { b'1' } else { b'0' };
                    assert!(!lei_ok(std::str::from_utf8(&bad).unwrap()));
                    return;
                }
            }
        }
        body.clear();
        panic!("no check digits found");
    }

    #[test]
    fn isin_known_good() {
        assert!(isin_ok("US0378331005")); // Apple — widely published
    }

    #[test]
    fn isin_rejects() {
        assert!(!isin_ok("US0378331004")); // flipped check digit
        assert!(!isin_ok("US037833100")); // 11 chars
    }

    #[test]
    fn luhn_known_good() {
        assert!(luhn_ok("79927398713")); // canonical Luhn example
        assert!(luhn_ok("7992739871 3"));
    }

    #[test]
    fn luhn_rejects() {
        assert!(!luhn_ok("79927398710"));
        assert!(!luhn_ok("123")); // too short
    }

    #[test]
    fn bic_shapes() {
        assert!(bic_ok("DEUTDEFF"));
        assert!(bic_ok("DEUTDEFF500"));
        assert!(!bic_ok("DEUTDE")); // too short
        assert!(!bic_ok("DEU1DEFF")); // digit in bank code
    }

    #[test]
    fn ean13_known_good() {
        assert!(ean13_ok("5901234123457")); // widely published example
        assert!(!ean13_ok("5901234123458")); // flipped check
    }

    #[test]
    fn gstin_self_consistent() {
        // construct a body, compute the checksum char, assert round-trip
        let body = "29ABCDE1234F1Z"; // 14 chars: state 29, PAN-shape, entity 1, Z
        let mut sum: u32 = 0;
        for (i, &c) in body.as_bytes().iter().enumerate() {
            let v = b36(c).unwrap();
            let f = if i % 2 == 0 { 1 } else { 2 };
            let p = v * f;
            sum += p / 36 + p % 36;
        }
        let check = (36 - (sum % 36)) % 36;
        let check_char = std::char::from_digit(check, 36).unwrap().to_ascii_uppercase();
        let gstin = format!("{body}{check_char}");
        assert_eq!(canonicalise_gstin(&gstin).unwrap(), gstin);
        // flipping the check char must fail
        let bad = format!("{body}{}", if check_char == '0' { '1' } else { '0' });
        assert_eq!(canonicalise_gstin(&bad).unwrap_err(), GstinError::InvalidChecksum);
    }

    #[test]
    fn gstin_rejects_bad_shapes() {
        assert_eq!(canonicalise_gstin("99ABCDE1234F1Z5").unwrap_err(), GstinError::BadStateCode);
        assert_eq!(canonicalise_gstin("29A1CDE1234F1Z5").unwrap_err(), GstinError::BadPanPattern);
        assert_eq!(canonicalise_gstin("29ABCDE1234F1A5").unwrap_err(), GstinError::BadEntityMarker);
    }
}
