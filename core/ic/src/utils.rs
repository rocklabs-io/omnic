
use std::collections::HashMap;

use ic_web3::types::H256;
use tiny_keccak::{Hasher, Keccak};

pub fn keccak256(msg: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut result = [0u8; 32];
    hasher.update(msg);
    hasher.finalize(&mut result);
    result
}

/// check if the roots match the criteria so far, return the check result and root
pub fn check_roots_result(roots: &HashMap<H256, usize>, total_result: usize) -> (bool, H256) {
    // when rpc fail, the root is vec![0; 32]
    if total_result <= 2 {
        // rpc len <= 2, all roots must match
        if roots.len() != 1 {
            return (false, H256::zero());
        } else {
            let r = roots.keys().next().unwrap().clone();
            return (r != H256::zero(), r);
        }
    } else {
        // rpc len > 2, half of the rpc result should be the same
        let limit = total_result / 2;
        // if contains > n/2 root, def fail
        let root_count = roots.keys().len();
        if root_count > limit {
            return (false, H256::zero());
        }

        // if #ZERO_HASH > n/2, def fail
        let error_count = roots.get(&H256::zero()).unwrap_or(&0);
        if error_count > &limit {
            return (false, H256::zero());
        }

        // if the #(root of most count) + #(rest rpc result) <= n / 2, def fail
        let mut possible_root = H256::zero();
        let mut possible_count: usize = 0;
        let mut current_count = 0;
        for (k ,v ) in roots {
            if v > &possible_count {
                possible_count = *v;
                possible_root = k.clone();
            }
            current_count += *v;
        }
        if possible_count + (total_result - current_count) <= limit {
            return (false, H256::zero());
        }

        // otherwise return true and root of most count
        return (true, possible_root.clone())
    }
}