use sha2::{Digest, Sha256};

pub fn sha256d(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(&first);

    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}

pub fn to_display_hex(hash: &[u8; 32]) -> String {
    let mut reversed = *hash;
    reversed.reverse();
    hex::encode(reversed)
}

pub fn to_hex(data: &[u8]) -> String {
    hex::encode(data)
}
