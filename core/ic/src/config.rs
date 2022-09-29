use candid::{Deserialize, CandidType};
use ic_web3::types::H256;

#[derive(CandidType, Deserialize, Clone)]
pub enum ChainType {
    EVM,
    Cosmos,
    Solana,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct ChainConfig {
    pub chain_type: ChainType,
    pub chain_id: u32,
    pub rpc_urls: Vec<String>, // multiple rpc providers
    pub omnic_addr: String, // omnic contract address on that chain
    pub omnic_start_block: u64, // omnic contract deployment block
}

impl ChainConfig {
    pub fn new(
        chain_type: ChainType,
        chain_id: u32, 
        rpc_urls: Vec<String>, 
        omnic_addr: String, 
        omnic_start_block: u64,
    ) -> ChainConfig {
        ChainConfig {
            chain_type: chain_type,
            chain_id: chain_id,
            rpc_urls: rpc_urls,
            omnic_addr: omnic_addr,
            omnic_start_block: omnic_start_block,
        }
    }

    pub fn add_rpc_url(&mut self, url: String) {
        self.rpc_urls.push(url);
    }
}