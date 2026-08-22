use std::fs;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

const CHUNK: usize = 1 << 20;

pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(nibble(byte >> 4));
        out.push(nibble(byte & 0x0f));
    }
    out
}

fn nibble(value: u8) -> char {
    const DIGITS: [u8; 16] = *b"0123456789abcdef";
    DIGITS[(value & 0x0f) as usize] as char
}

pub fn of_bytes(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub fn short(bytes: &[u8], keep: usize) -> String {
    let mut digest = of_bytes(bytes);
    digest.truncate(keep * 2);
    digest
}

pub fn of_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK];

    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }

    Ok(hex(&hasher.finalize()))
}

#[cfg(test)]
#[path = "tests/hash.rs"]
mod tests;
