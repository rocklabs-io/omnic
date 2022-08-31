
/*
    used in omnic proxy canister
    store merkle roots fetched from corresponding chain
*/

use std::collections::{HashMap, VecDeque};
use ic_web3::types::H256;
use crate::chain_config::ChainConfig;
use crate::Message;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ChainRoots {
    pub config: ChainConfig,
    pub roots: HashMap<H256, u64>, // root hash -> confirm time
}

impl ChainRoots {
    pub fn set_current_block(&mut self, v: u64) {
        self.config.set_current_block(v);
    }

    pub fn set_batch_size(&mut self, v: u64) {
        self.config.set_batch_size(v);
    }

    fn insert_root(&mut self, root: H256, confirm_at: u64) {
        // check if exist
        if !self.roots.contains_key(root) {
            self.roots.insert(root, confirm_at);
        }
    }
}