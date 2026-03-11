use crate::analysis::classify::classify_output;

pub fn derive_address(script_pubkey: &[u8]) -> Option<String> {
    let script_type = classify_output(script_pubkey);

    match script_type {
        "p2pkh" => {
            let hash = &script_pubkey[3..23];
            Some(base58check_encode(0x00, hash))
        }

        "p2sh" => {
            let hash = &script_pubkey[2..22];
            Some(base58check_encode(0x05, hash))
        }

        "p2wpkh" => {
            let program = &script_pubkey[2..22];
            Some(bech32_encode(0, program))
        }

        "p2wsh" => {
            let program = &script_pubkey[2..34];
            Some(bech32_encode(0, program))
        }

        "p2tr" => {
            let program = &script_pubkey[2..34];
            Some(bech32m_encode(1, program))
        }

        _ => None,
    }
}

fn base58check_encode(version: u8, payload: &[u8]) -> String {
    let mut data = Vec::with_capacity(1 + payload.len());
    data.push(version);
    data.extend_from_slice(payload);
    bs58::encode(data).with_check().into_string()
}

fn bech32_encode(witness_version: u8, program: &[u8]) -> String {
    let hrp = bech32::Hrp::parse("bc").expect("valid hrp");
    bech32::segwit::encode(
        hrp,
        bech32::Fe32::try_from(witness_version).expect("valid witness version"),
        program,
    )
    .expect("valid bech32 encoding")
}

fn bech32m_encode(witness_version: u8, program: &[u8]) -> String {
    let hrp = bech32::Hrp::parse("bc").expect("valid hrp");
    bech32::segwit::encode(
        hrp,
        bech32::Fe32::try_from(witness_version).expect("valid witness version"),
        program,
    )
    .expect("valid bech32m encoding")
}
