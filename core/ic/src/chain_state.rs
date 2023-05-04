use candid::{CandidType, Deserialize};
use crate::config::{ChainConfig, ChainType};
use crate::types::MessageStable;

use std::cmp;

#[derive(CandidType, Deserialize, Default)]
pub struct ChainState {
    pub config: ChainConfig,
    pub canister_addr: String, // the address controlled by the proxy canister on this chain
    pub last_scanned_block: u64,
    pub events: Vec<([u8;32], MessageStable)>, // messageHash => message
    // pub txs: Vec<Message>, // outgoging txs
}

impl ChainState {
    pub fn new(
        chain_config: ChainConfig,
    ) -> ChainState {
        let start_block = chain_config.omnic_start_block;
        ChainState {
            config: chain_config,
            canister_addr: "".into(),
            last_scanned_block: start_block,
            events: Default::default()
        }
    }

    pub fn update_config(&mut self, new_config: ChainConfig) {
        self.config = new_config;
    }

    pub fn update_last_scanned_block(&mut self, block_num: u64) {
        self.last_scanned_block = block_num;
    }

    pub fn get_suggested_start_block(&self) -> u64 {
        cmp::max(self.last_scanned_block - 10, 0)
    }

    pub fn add_urls(&mut self, urls: Vec<String>) {
        self.config.add_urls(urls);
    }

    pub fn rpc_urls(&self) -> Vec<String> {
        self.config.rpc_urls.clone()
    }

    pub fn chain_type(&self) -> ChainType {
        self.config.chain_type.clone()
    }

    pub fn set_canister_addr(&mut self, addr: String) {
        self.canister_addr = addr;
    }

    pub fn get_messages(&self, start: u64, limit: u64) -> Vec<MessageStable>{

        let end = cmp::min(start + limit, self.events.len() as u64);
        let mut events: Vec<MessageStable> = self.events.iter().map(|(x, y)| y.clone()).collect();
        events.truncate(end as usize);
        let res: Vec<MessageStable> = events.into_iter().skip(start as usize).collect();
        res
    }

    pub fn get_message_by_hash(&self, hash: &[u8]) -> Vec<MessageStable> {
        self.events.iter().filter(|e| e.0 == hash).map(|(_,y)| y.clone()).collect()
    }
}