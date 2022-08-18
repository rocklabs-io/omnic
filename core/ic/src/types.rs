use std::collections::HashSet;
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Int, Nat};
use ic_cdk::export::Principal;

#[derive(CandidType, Deserialize, Clone)]
pub struct InitArgs {
    pub owner: Option<Principal> // if None, caller will be the owner
}

/*
struct MessageFormat {
        uint32 _srcChainId;
        bytes32 _srcSenderAddress;
        uint32 _nonce;
        uint32 _dstChainId;
        bytes32 _recipientAddress;
        bytes payload;
    }
*/

#[derive(CandidType, Deserialize, Clone)]
pub struct ChainConfig {
    pub chain_id: u32,
    pub rpc_url: String,
    // omnic contract address on that chain
    pub omnic_addr: Vec<u8>,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct Message {
    pub src_chain: u32,
    pub src_sender: Vec<u8>,
    pub nonce: u32,
    pub dst_chain: u32,
    pub recipient: Vec<u8>,
    pub payload: Vec<u8>
}

#[derive(CandidType, Deserialize, Clone)]
pub enum Task {
    GetLogs,
    SendTx
}
