use std::collections::VecDeque;
use ic_web3::types::H256;
use candid::{CandidType, Deserialize};
use crate::config::{ChainConfig, ChainType};

#[derive(CandidType, Deserialize, Clone, Default)]
pub struct ChainState {
    pub config: ChainConfig,
    pub roots: VecDeque<Vec<u8>>,
    pub next_index: u32, // leaf_index for next message
    pub canister_addr: String, // the address controlled by the proxy canister on this chain
    // pub txs: Vec<Message>, // outgoging txs
}

impl ChainState {
    pub fn new(
        chain_config: ChainConfig,
    ) -> ChainState {
        ChainState {
            config: chain_config,
            roots: VecDeque::new(),
            next_index: 0,
            canister_addr: "".into(),
        }
    }

    pub fn update_config(&mut self, new_config: ChainConfig) {
        self.config = new_config;
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

    pub fn bump_index(&mut self) {
        self.next_index += 1;
    }

    pub fn next_index(&self) -> u32 {
        self.next_index
    }

    pub fn insert_root(&mut self, r: H256) {
        let root = r.as_bytes().to_vec();
        if !self.roots.contains(&root) {
            self.roots.push_back(root);
        }
    }

    pub fn is_root_exist(&self, r: H256) -> bool {
        let root = r.as_bytes().to_vec();
        self.roots.contains(&root)
    }

    pub fn latest_root(&self) -> H256 {
        match self.roots.back() {
            Some(v) => H256::from_slice(&v),
            None => H256::zero(),
        }
    }

    pub fn all_roots(&self) -> Vec<H256> {
        self.roots.iter().map(|r| {
            H256::from_slice(&r)
        }).collect()
    }
}