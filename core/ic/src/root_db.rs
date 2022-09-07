// merkle root storage, for proxy canister

use std::collections::VecDeque;
use ic_web3::types::H256;

use crate::types::{Message, RawMessage};
use crate::error::OmnicError;


pub struct RootDB {
    pub roots: VecDeque<(H256, u64)>,
    pub optimistic_delay: u64,
}

impl RootDB {
    pub fn new() -> Self {
        RootDB {
            msgs: VecDeque::new(),
            optimistic_delay: 1800, // 30 mins
        }
    }

    pub fn set_optimistic_delay(&mut self, delay: u64) {
        self.optimistic_delay = delay;
    }

    pub fn insert_root(&mut self, root: H256, confirm_at: u64) {

    }

    // get the latest root
    pub fn latest_root(&self) -> H256 {

    }

    // get the latest root that has passed the optimistic verification challenge period
    // now - confirmAt >= 30 mins
    pub fn latest_op_root(&self, now: u64) -> H256 {

    }

}