use crate::state::DetailValue;

use std::collections::HashMap;
use crate::types::{Message, MessageStable};
use ic_web3::types::{H256, Log};
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

pub fn check_scan_message_results(messages: &HashMap<usize, Vec<MessageStable>>, rpcs_count: usize) -> (bool, Vec<MessageStable>) {
    // compare each message for different rpc 
    // what should be checked?
    // messageHash ?= keccak256(msg.body)
    // message block number is same?
    // others...
    (false, vec![])
}


// pub struct Log {
//     /// H160
//     pub address: H160,
//     /// Topics
//     pub topics: Vec<H256>,
//     /// Data
//     pub data: Bytes,
//     /// Block Hash
//     #[serde(rename = "blockHash")]
//     pub block_hash: Option<H256>,
//     /// Block Number
//     #[serde(rename = "blockNumber")]
//     pub block_number: Option<U64>,
//     /// Transaction Hash
//     #[serde(rename = "transactionHash")]
//     pub transaction_hash: Option<H256>,
//     /// Transaction Index
//     #[serde(rename = "transactionIndex")]
//     pub transaction_index: Option<Index>,
//     /// Log Index in Block
//     #[serde(rename = "logIndex")]
//     pub log_index: Option<U256>,
//     /// Log Index in Transaction
//     #[serde(rename = "transactionLogIndex")]
//     pub transaction_log_index: Option<U256>,
//     /// Log Type
//     #[serde(rename = "logType")]
//     pub log_type: Option<String>,
//     /// Removed
//     pub removed: Option<bool>,
// }

pub fn decode_log(logs: Vec<Log>) -> Vec<MessageStable> {
    // todo: decode log to MessageStable
    logs.into_iter().map(|l| {
        let m = Message::from_raw(l.data.0).unwrap();
        MessageStable::from(m)
    }).collect()
}

/// Allows creating details for an event.
#[derive(Default, Clone)]
pub struct DetailsBuilder {
    inner: Vec<(String, DetailValue)>,
}

impl DetailsBuilder {
    /// Creates a new, empty builder.
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new element.
    #[inline(always)]
    pub fn insert(mut self, key: impl Into<String>, value: impl Into<DetailValue>) -> Self {
        self.inner.push((key.into(), value.into()));

        self
    }

    #[inline(always)]
    pub fn build(self) -> Vec<(String, DetailValue)> {
        self.inner
    }
}