/// Decode BIP34 height from coinbase scriptSig.
/// First byte = length N, next N bytes = height (little-endian).
pub fn decode_bip34_height(script_sig: &[u8]) -> u64 {
    if script_sig.is_empty() {
        return 0;
    }

    let n = script_sig[0] as usize;

    if n == 0 {
        return 0;
    }

    if n >= 1 && n <= 75 && script_sig.len() >= 1 + n {
        let mut height: u64 = 0;
        for i in 0..n {
            height |= (script_sig[1 + i] as u64) << (8 * i);
        }
        return height;
    }

    0
}
