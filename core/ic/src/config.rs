use candid::{Deserialize, CandidType, Principal};

#[derive(CandidType, Deserialize, Clone)]
pub enum ChainType {
    Evm,
    Cosmos,
    Solana,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct ChainConfig {
    pub chain_type: ChainType,
    pub chain_id: u32,
    pub rpc_urls: Vec<String>, // multiple rpc providers
    pub gateway_addr: Principal, // gateway canister address
    pub omnic_addr: String, // omnic contract address on that chain
    pub omnic_start_block: u64, // omnic contract deployment block
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self { 
            chain_type: ChainType::Evm, 
            chain_id: Default::default(), 
            rpc_urls: Default::default(), 
            gateway_addr: Principal::anonymous(), 
            omnic_addr: Default::default(), 
            omnic_start_block: Default::default()
        }
    }
}

impl ChainConfig {
    pub fn new(
        chain_type: ChainType,
        chain_id: u32, 
        rpc_urls: Vec<String>,
        gateway_addr: Principal, 
        omnic_addr: String, 
        omnic_start_block: u64,
    ) -> ChainConfig {
        ChainConfig {
            chain_type: chain_type,
            chain_id: chain_id,
            rpc_urls: rpc_urls,
            gateway_addr: gateway_addr,
            omnic_addr: omnic_addr,
            omnic_start_block: omnic_start_block,
        }
    }

    pub fn add_rpc_url(&mut self, url: String) {
        self.rpc_urls.push(url);
    }

    pub fn add_urls(&mut self, urls: Vec<String>) {
        self.rpc_urls.extend(urls);
    }
}