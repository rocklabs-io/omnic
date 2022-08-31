

/*
    chain config
*/

#[derive(CandidType, Deserialize, Clone, Debug)]
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
            current_block: current_block,
            batch_size: if let Some(v) = batch_size { v } else { 1000 },
        }
    }

    pub fn set_current_block(&mut self, v: u64) {
        self.current_block = v;
    }

    pub fn set_batch_size(&mut self, v: u64) {
        self.batch_size = v;
    }
}