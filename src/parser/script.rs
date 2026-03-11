pub fn disassemble(script: &[u8]) -> String {
    let mut parts = Vec::new();
    let mut i = 0;

    while i < script.len() {
        let op = script[i];
        i += 1;

        match op {
            0x01..=0x4b => {
                let n = op as usize;
                if i + n <= script.len() {
                    parts.push(format!("OP_PUSHBYTES_{}", n));
                    parts.push(hex::encode(&script[i..i + n]));
                    i += n;
                } else {
                    parts.push(format!("OP_PUSHBYTES_{}", n));
                    parts.push(hex::encode(&script[i..]));
                    break;
                }
            }
            0x4c => {
                if i < script.len() {
                    let n = script[i] as usize;
                    i += 1;
                    parts.push("OP_PUSHDATA1".to_string());
                    if i + n <= script.len() {
                        parts.push(hex::encode(&script[i..i + n]));
                        i += n;
                    } else {
                        parts.push(hex::encode(&script[i..]));
                        break;
                    }
                }
            }
            0x4d => {
                if i + 2 <= script.len() {
                    let n = u16::from_le_bytes([script[i], script[i + 1]]) as usize;
                    i += 2;
                    parts.push("OP_PUSHDATA2".to_string());
                    if i + n <= script.len() {
                        parts.push(hex::encode(&script[i..i + n]));
                        i += n;
                    } else {
                        parts.push(hex::encode(&script[i..]));
                        break;
                    }
                }
            }
            0x4e => {
                if i + 4 <= script.len() {
                    let n = u32::from_le_bytes([
                        script[i],
                        script[i + 1],
                        script[i + 2],
                        script[i + 3],
                    ]) as usize;
                    i += 4;
                    parts.push("OP_PUSHDATA4".to_string());
                    if i + n <= script.len() {
                        parts.push(hex::encode(&script[i..i + n]));
                        i += n;
                    } else {
                        parts.push(hex::encode(&script[i..]));
                        break;
                    }
                }
            }
            _ => {
                parts.push(opcode_name(op).to_string());
            }
        }
    }

    parts.join(" ")
}

fn opcode_name(op: u8) -> &'static str {
    match op {
        0x00 => "OP_0",
        0x4f => "OP_1NEGATE",
        0x51 => "OP_1",
        0x52 => "OP_2",
        0x53 => "OP_3",
        0x54 => "OP_4",
        0x55 => "OP_5",
        0x56 => "OP_6",
        0x57 => "OP_7",
        0x58 => "OP_8",
        0x59 => "OP_9",
        0x5a => "OP_10",
        0x5b => "OP_11",
        0x5c => "OP_12",
        0x5d => "OP_13",
        0x5e => "OP_14",
        0x5f => "OP_15",
        0x60 => "OP_16",
        0x61 => "OP_NOP",
        0x63 => "OP_IF",
        0x64 => "OP_NOTIF",
        0x67 => "OP_ELSE",
        0x68 => "OP_ENDIF",
        0x69 => "OP_VERIFY",
        0x6a => "OP_RETURN",
        0x6b => "OP_TOALTSTACK",
        0x6c => "OP_FROMALTSTACK",
        0x73 => "OP_IFDUP",
        0x74 => "OP_DEPTH",
        0x75 => "OP_DROP",
        0x76 => "OP_DUP",
        0x77 => "OP_NIP",
        0x78 => "OP_OVER",
        0x79 => "OP_PICK",
        0x7a => "OP_ROLL",
        0x7b => "OP_ROT",
        0x7c => "OP_SWAP",
        0x7d => "OP_TUCK",
        0x6d => "OP_2DROP",
        0x6e => "OP_2DUP",
        0x6f => "OP_3DUP",
        0x70 => "OP_2OVER",
        0x71 => "OP_2ROT",
        0x72 => "OP_2SWAP",
        0x7e => "OP_CAT",
        0x82 => "OP_SIZE",
        0x87 => "OP_EQUAL",
        0x88 => "OP_EQUALVERIFY",
        0x8b => "OP_1ADD",
        0x8c => "OP_1SUB",
        0x8f => "OP_NEGATE",
        0x90 => "OP_ABS",
        0x91 => "OP_NOT",
        0x92 => "OP_0NOTEQUAL",
        0x93 => "OP_ADD",
        0x94 => "OP_SUB",
        0x9a => "OP_BOOLAND",
        0x9b => "OP_BOOLOR",
        0x9c => "OP_NUMEQUAL",
        0x9d => "OP_NUMEQUALVERIFY",
        0x9e => "OP_NUMNOTEQUAL",
        0x9f => "OP_LESSTHAN",
        0xa0 => "OP_GREATERTHAN",
        0xa1 => "OP_LESSTHANOREQUAL",
        0xa2 => "OP_GREATERTHANOREQUAL",
        0xa3 => "OP_MIN",
        0xa4 => "OP_MAX",
        0xa5 => "OP_WITHIN",
        0xa6 => "OP_RIPEMD160",
        0xa7 => "OP_SHA1",
        0xa8 => "OP_SHA256",
        0xa9 => "OP_HASH160",
        0xaa => "OP_HASH256",
        0xab => "OP_CODESEPARATOR",
        0xac => "OP_CHECKSIG",
        0xad => "OP_CHECKSIGVERIFY",
        0xae => "OP_CHECKMULTISIG",
        0xaf => "OP_CHECKMULTISIGVERIFY",
        0xb1 => "OP_CHECKLOCKTIMEVERIFY",
        0xb2 => "OP_CHECKSEQUENCEVERIFY",
        0xb0 => "OP_NOP1",
        0xb3 => "OP_NOP4",
        0xb4 => "OP_NOP5",
        0xb5 => "OP_NOP6",
        0xb6 => "OP_NOP7",
        0xb7 => "OP_NOP8",
        0xb8 => "OP_NOP9",
        0xb9 => "OP_NOP10",
        0xba => "OP_CHECKSIGADD",
        _ => "OP_UNKNOWN",
    }
}
