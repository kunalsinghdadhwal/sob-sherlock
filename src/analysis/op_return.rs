pub fn extract_op_return_data(script_pubkey: &[u8]) -> Vec<u8> {
    if script_pubkey.is_empty() || script_pubkey[0] != 0x6a {
        return Vec::new();
    }

    let mut data = Vec::new();
    let mut i = 1;

    while i < script_pubkey.len() {
        let op = script_pubkey[i];
        i += 1;

        match op {
            0x01..=0x4b => {
                let n = op as usize;
                if i + n <= script_pubkey.len() {
                    data.extend_from_slice(&script_pubkey[i..i + n]);
                    i += n;
                } else {
                    data.extend_from_slice(&script_pubkey[i..]);
                    break;
                }
            }
            0x4c => {
                if i < script_pubkey.len() {
                    let n = script_pubkey[i] as usize;
                    i += 1;
                    if i + n <= script_pubkey.len() {
                        data.extend_from_slice(&script_pubkey[i..i + n]);
                        i += n;
                    } else {
                        data.extend_from_slice(&script_pubkey[i..]);
                        break;
                    }
                }
            }
            0x4d => {
                if i + 2 <= script_pubkey.len() {
                    let n = u16::from_le_bytes([script_pubkey[i], script_pubkey[i + 1]]) as usize;
                    i += 2;
                    if i + n <= script_pubkey.len() {
                        data.extend_from_slice(&script_pubkey[i..i + n]);
                        i += n;
                    } else {
                        data.extend_from_slice(&script_pubkey[i..]);
                        break;
                    }
                }
            }
            0x00 => {}
            _ => break,
        }
    }

    data
}

pub fn detect_protocol(data: &[u8]) -> &'static str {
    if data.len() >= 4 && data[..4] == [0x6f, 0x6d, 0x6e, 0x69] {
        return "omni";
    }
    if data.len() >= 5 && data[..5] == [0x01, 0x09, 0xf9, 0x11, 0x02] {
        return "opentimestamps";
    }

    "unknown"
}

pub fn try_utf8(data: &[u8]) -> Option<String> {
    std::str::from_utf8(data).ok().map(|s| s.to_string())
}
