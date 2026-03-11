pub fn classify_output(script_pubkey: &[u8]) -> &'static str {
    let len = script_pubkey.len();

    // OP_RETURN: starts with 0x6a
    if len >= 1 && script_pubkey[0] == 0x6a {
        return "op_return";
    }

    // P2PKH: OP_DUP OP_HASH160 OP_PUSHBYTES_20 [20] OP_EQUALVERIFY OP_CHECKSIG
    if len == 25
        && script_pubkey[0] == 0x76
        && script_pubkey[1] == 0xa9
        && script_pubkey[2] == 0x14
        && script_pubkey[23] == 0x88
        && script_pubkey[24] == 0xac
    {
        return "p2pkh";
    }

    // P2SH: OP_HASH160 OP_PUSHBYTES_20 [20] OP_EQUAL
    if len == 23
        && script_pubkey[0] == 0xa9
        && script_pubkey[1] == 0x14
        && script_pubkey[22] == 0x87
    {
        return "p2sh";
    }

    // P2WPKH: OP_0 OP_PUSHBYTES_20 [20]
    if len == 22 && script_pubkey[0] == 0x00 && script_pubkey[1] == 0x14 {
        return "p2wpkh";
    }

    // P2WSH: OP_0 OP_PUSHBYTES_32 [32]
    if len == 34 && script_pubkey[0] == 0x00 && script_pubkey[1] == 0x20 {
        return "p2wsh";
    }

    // P2TR: OP_1 OP_PUSHBYTES_32 [32]
    if len == 34 && script_pubkey[0] == 0x51 && script_pubkey[1] == 0x20 {
        return "p2tr";
    }

    "unknown"
}

pub fn classify_input(
    prevout_script_pubkey: &[u8],
    script_sig: &[u8],
    witness: &[Vec<u8>],
) -> &'static str {
    let out_type = classify_output(prevout_script_pubkey);

    match out_type {
        "p2pkh" => "p2pkh",
        "p2wpkh" => "p2wpkh",
        "p2wsh" => "p2wsh",

        "p2tr" => {
            if witness.len() == 1 && (witness[0].len() == 64 || witness[0].len() == 65) {
                "p2tr_keypath"
            } else if witness.len() >= 2 {
                let last = &witness[witness.len() - 1];
                if !last.is_empty() && (last[0] == 0xc0 || last[0] == 0xc1) {
                    "p2tr_scriptpath"
                } else {
                    "p2tr_keypath"
                }
            } else {
                "unknown"
            }
        }

        "p2sh" => {
            if !script_sig.is_empty() && !witness.is_empty() {
                let push_len = script_sig[0] as usize;
                if push_len + 1 == script_sig.len() && push_len >= 2 {
                    let redeem = &script_sig[1..];
                    if redeem.len() == 22 && redeem[0] == 0x00 && redeem[1] == 0x14 {
                        return "p2sh-p2wpkh";
                    }
                    if redeem.len() == 34 && redeem[0] == 0x00 && redeem[1] == 0x20 {
                        return "p2sh-p2wsh";
                    }
                }
            }
            "unknown"
        }

        _ => "unknown",
    }
}
