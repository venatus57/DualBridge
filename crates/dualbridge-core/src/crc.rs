//! CRC32 used by PlayStation controllers over Bluetooth.
//!
//! It is the standard CRC-32 (IEEE 802.3, reflected polynomial `0xEDB88320`),
//! computed over a one-byte "seed" (the Bluetooth HID transaction header)
//! followed by the report bytes. The result is stored little-endian in the last
//! four bytes of the report.

/// HID header for input reports (DATA | INPUT).
pub const SEED_INPUT: u8 = 0xA1;
/// HID header for output reports (DATA | OUTPUT).
pub const SEED_OUTPUT: u8 = 0xA2;
/// HID header for feature reports read from the device (DATA | FEATURE).
pub const SEED_FEATURE: u8 = 0xA3;

const TABLE: [u32; 256] = build_table();

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

#[inline]
fn update(mut crc: u32, data: &[u8]) -> u32 {
    for &b in data {
        crc = TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc
}

/// Standard CRC-32 of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    !update(!0, data)
}

/// CRC-32 of `seed` followed by `data`.
pub fn crc32_seeded(seed: u8, data: &[u8]) -> u32 {
    !update(update(!0, &[seed]), data)
}

/// Checks the CRC stored in the last four bytes of a Bluetooth report.
pub fn verify(seed: u8, report: &[u8]) -> bool {
    if report.len() < 5 {
        return false;
    }
    let (body, tail) = report.split_at(report.len() - 4);
    let stored = u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
    crc32_seeded(seed, body) == stored
}

/// Writes the CRC of everything before the last four bytes into those bytes.
pub fn sign(seed: u8, report: &mut [u8]) {
    let n = report.len();
    debug_assert!(n >= 5);
    let crc = crc32_seeded(seed, &report[..n - 4]);
    report[n - 4..].copy_from_slice(&crc.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        // The classic CRC-32 check value.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn seeded_equals_prefixed() {
        let data = [1u8, 2, 3, 4, 5, 6];
        let mut prefixed = vec![SEED_OUTPUT];
        prefixed.extend_from_slice(&data);
        assert_eq!(crc32_seeded(SEED_OUTPUT, &data), crc32(&prefixed));
    }

    #[test]
    fn sign_then_verify() {
        let mut report = [0u8; 78];
        report[0] = 0x31;
        report[10] = 0x42;
        sign(SEED_OUTPUT, &mut report);
        assert!(verify(SEED_OUTPUT, &report));
        report[10] = 0x43;
        assert!(!verify(SEED_OUTPUT, &report));
    }
}
