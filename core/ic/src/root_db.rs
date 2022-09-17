// merkle root storage, for proxy canister

use std::collections::BTreeMap;
use ic_web3::types::H256;

pub struct RootDB {
    pub roots_value: BTreeMap<u64, H256>, // confirmed time -> hash value
    pub roots_confirmed: BTreeMap<H256, u64>, // hash value -> confirmed time
    pub optimistic_delay: u64,
}

impl RootDB {
    pub fn new() -> Self {
        RootDB {
            optimistic_delay: 1800, // 30 mins
            roots_value: BTreeMap::new(),
            roots_confirmed: BTreeMap::new(),
        }
    }

    pub fn set_optimistic_delay(&mut self, delay: u64) {
        self.optimistic_delay = delay;
    }

    pub fn insert_root(&mut self, root: H256, confirm_at: u64) {
        // insert if not exist
        self.roots_value.entry(confirm_at).or_insert(root);
        self.roots_confirmed.entry(root).or_insert(confirm_at);
    }

    // get the latest root
    pub fn latest_root(&self) -> H256 {
        // get from roots_value with biggest key
        self.roots_value.iter().next_back().unwrap_or((&0, &H256::default())).1.to_owned()
    }

    // get the latest root that has passed the optimistic verification challenge period
    // now - confirmAt >= 30 mins
    pub fn latest_op_root(&self, now: u64) -> H256 {
        self.roots_value
            .range(..=(now - self.optimistic_delay))
            .next_back()
            .unwrap_or((&0, &H256::default()))
            .1
            .to_owned()
    }

    pub fn is_root_exist(&self, root: &H256) -> bool {
        self.roots_confirmed.contains_key(root)
    }

    pub fn get_root_confirmed(&self, root: &H256) -> u64 {
        self.roots_confirmed.get(root).unwrap_or(&u64::max_value()).to_owned()
    }

}