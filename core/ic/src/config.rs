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
    pub max_waiting_time: u64, // send msgs once reaching max_waiting_time
	pub max_cache_msg: u64, // send msgs once reaching max_cache capability
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self { 
            chain_type: ChainType::Evm, 
            chain_id: Default::default(), 
            rpc_urls: Default::default(), 
            gateway_addr: Principal::anonymous(), 
            omnic_addr: Default::default(), 
            omnic_start_block: Default::default(),
            max_waiting_time: 0u64,
            max_cache_msg: 0u64
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
        max_waiting_time: u64,
        max_cache_msg: u64
    ) -> ChainConfig {
        ChainConfig {
            chain_type,
            chain_id,
            rpc_urls,
            gateway_addr,
            omnic_addr,
            omnic_start_block,
            max_waiting_time,
            max_cache_msg,
        }
    }

    pub fn add_rpc_url(&mut self, url: String) {
        self.rpc_urls.push(url);
    }

    pub fn add_urls(&mut self, urls: Vec<String>) {
        self.rpc_urls.extend(urls);
    }

    pub fn get_cache_info(&self) -> (u64, u64) {
        (self.max_waiting_time, self.max_cache_msg)
    }
}