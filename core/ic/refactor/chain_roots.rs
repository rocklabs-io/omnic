
/*
    used in omnic proxy canister
    store merkle roots fetched from corresponding chain
*/

use std::collections::{HashMap, VecDeque};
use ic_web3::types::H256;
use crate::chain_config::ChainConfig;
use crate::Message;

#[derive(Clone, Debug)]
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

    pub fn insert_root(&mut self, root: H256, confirm_at: u64) {
        // check if exist
        if !self.roots.contains_key(&root) {
            self.roots.insert(root, confirm_at);
        }
    }

    pub fn is_root_exist(&self, root: H256) -> bool {
        self.roots.contains_key(&root)
    }

    pub fn is_root_valid(&self, root: H256, ts: u64) -> bool {
        self.roots.get(&root).map_or(false, |c| c.clone() <= ts)
    }
}