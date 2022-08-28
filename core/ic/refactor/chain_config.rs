


#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ChainConfig {
    pub chain_id: u32,
    pub rpc_url: String,
    pub omnic_addr: String, // omnic contract address on that chain
    pub omnic_start_block: u64, // omnic contract deployment block
    pub current_block: u64, // current block getLogs scanned, init value = omnic_start_block
    pub batch_size: u64, // how many blocks each getLogs scan
}

impl ChainConfig {
    
}