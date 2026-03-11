use std::fs;
use std::io;

pub fn read_xor_key(path: &str) -> io::Result<Vec<u8>> {
    fs::read(path)
}

pub fn xor_decode(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() || key.iter().all(|&b| b == 0) {
        return data.to_vec();
    }
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect()
}
