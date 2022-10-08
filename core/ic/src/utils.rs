
use tiny_keccak::{Hasher, Keccak};

pub fn keccak256(msg: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut result = [0u8; 32];
    hasher.update(msg);
    hasher.finalize(&mut result);
    result
}