use candid::Deserialize;
use ic_web3::types::H256;

use crate::RootDB;

#[derive(Clone, Debug)]
pub struct IndexerConfig {
    pub chain_id: u32,
    pub rpc_url: String,
    pub omnic_addr: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ChainConfig {
    pub chain_id: u32,
    pub rpc_urls: Vec<String>, // multiple rpc providers
    pub omnic_addr: String, // omnic contract address on that chain
    pub omnic_start_block: u64, // omnic contract deployment block
    pub current_block: u64, // current block getLogs scanned, init value = omnic_start_block
    pub batch_size: u64, // how many blocks each getLogs scan
}

impl ChainConfig {
    pub fn new(
        chain_id: u32, 
        rpc_urls: Vec<String>, 
        omnic_addr: String, 
        omnic_start_block: u64,
        batch_size: Option<u64>
    ) -> ChainConfig {
        ChainConfig {
            chain_id: chain_id,
            rpc_urls: rpc_urls,
            omnic_addr: omnic_addr,
            omnic_start_block: omnic_start_block,
            current_block: omnic_start_block,
            batch_size: if let Some(v) = batch_size { v } else { 1000 },
        }
    }

    pub fn set_current_block(&mut self, v: u64) {
        self.current_block = v;
    }

    pub fn set_batch_size(&mut self, v: u64) {
        self.batch_size = v;
    }

    pub fn add_rpc_url(&mut self, url: String) {
        self.rpc_urls.push(url);
    }
}

pub struct ChainRoots {
    pub config: ChainConfig,
    pub roots: RootDB, // root hash -> confirm time
}

impl ChainRoots {
    pub fn set_current_block(&mut self, v: u64) {
        self.config.set_current_block(v);
    }

    pub fn set_batch_size(&mut self, v: u64) {
        self.config.set_batch_size(v);
    }

    pub fn insert_root(&mut self, root: H256, confirm_at: u64) {
        self.roots.insert_root(root, confirm_at);
    }

    pub fn is_root_exist(&self, root: H256) -> bool {
        self.roots.is_root_exist(&root)
    }

    pub fn is_root_valid(&self, root: H256, ts: u64) -> bool {
        let confirm_at = self.roots.get_root_confirmed(&root);
        confirm_at <= ts
    }

    pub fn latest_root(&self) -> H256 {
        // get from roots_value with biggest key
        self.roots.latest_root()
    }

    // get the latest root that has passed the optimistic verification challenge period
    // now - confirmAt >= 30 mins
    pub fn latest_op_root(&self, now: u64) -> H256 {
        self.roots.latest_op_root(now)
    }
}