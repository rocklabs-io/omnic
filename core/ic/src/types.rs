use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Int, Nat};
use ic_cdk::export::Principal;
use ic_web3::ethabi::{Event, EventParam, ParamType, RawLog};
use ic_web3::types::Log;
use std::convert::TryInto;
use std::fmt;

#[derive(CandidType, Deserialize, Clone)]
pub struct InitArgs {
    pub owner: Option<Principal> // if None, caller will be the owner
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ChainConfig {
    pub chain_id: u32,
    pub rpc_url: String,
    pub omnic_addr: String, // omnic contract address on that chain
    pub omnic_start_block: u64, // omnic contract deployment block
    pub current_block: u64, // current block getLogs scanned, init value = omnic_start_block
    pub batch_size: u64, // how many blocks each getLogs scan
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Message {
    pub hash: Vec<u8>,
    pub src_chain: u32,
    pub src_sender: Vec<u8>,
    pub nonce: u32,
    pub dst_chain: u32,
    pub recipient: Vec<u8>,
    pub payload: Vec<u8>
}

impl Message {
    pub fn from_log(log: &Log) -> Result<Message, String> {
        let params = vec![
            EventParam { name: "messageHash".to_string(), kind: ParamType::FixedBytes(32), indexed: true },
            EventParam { name: "dstNonce".to_string(), kind: ParamType::Uint(32), indexed: true },
            EventParam { name: "srcChainId".to_string(), kind: ParamType::Uint(32), indexed: false },
            EventParam { name: "srcSenderAddress".to_string(), kind: ParamType::FixedBytes(32), indexed: false },
            EventParam { name: "dstChainId".to_string(), kind: ParamType::Uint(32), indexed: false },
            EventParam { name: "recipient".to_string(), kind: ParamType::FixedBytes(32), indexed: false },
            EventParam { name: "data".to_string(), kind: ParamType::Bytes, indexed: false }
        ];

        let event = Event {
            name: "EnqueueMessage".to_string(),
            inputs: params,
            anonymous: false
        };
        let res = event.parse_log(RawLog {
            topics: log.topics.clone(),
            data: log.data.clone().0
        }).map_err(|e| format!("ethabi parse_log failed: {}", e))?;
        
        let msg_hash = res.params.iter().find(|p| p.name == "messageHash").ok_or("missing messgaHash".to_string())?;
        let dst_nonce = res.params.iter().find(|p| p.name == "dstNonce").ok_or("missing dstNonce".to_string())?;
        let src_chain = res.params.iter().find(|p| p.name == "srcChainId").ok_or("missing srcChainId".to_string())?;
        let src_sender = res.params.iter().find(|p| p.name == "srcSenderAddress").ok_or("missing srcSenderAddress".to_string())?;
        let dst_chain = res.params.iter().find(|p| p.name == "dstChainId").ok_or("missing dstChainId".to_string())?;
        let recipient = res.params.iter().find(|p| p.name == "recipient").ok_or("missing recipient".to_string())?;
        let payload = res.params.iter().find(|p| p.name == "data").ok_or("missing data".to_string())?;
        // ic_cdk::println!("event: {:?}", res);
        // ic_cdk::println!("msg_hash: {:?}", msg_hash.value.clone());

        Ok(Message {
            hash: msg_hash.value.clone().into_fixed_bytes().ok_or("can not convert hash to bytes")?,
            src_chain: src_chain.value.clone().into_uint().ok_or("can not convert src_chain to U256")?.try_into().map_err(|_| format!("convert U256 to u32 failed"))?,
            src_sender: src_sender.value.clone().into_fixed_bytes().ok_or("can not src_sender to bytes")?,
            nonce: dst_nonce.value.clone().into_uint().ok_or("can not convert nonce to U256")?.try_into().map_err(|_| format!("convert U256 to u32 failed"))?,
            dst_chain: dst_chain.value.clone().into_uint().ok_or("can not convert dst_chain to U256")?.try_into().map_err(|_| format!("convert U256 to u32 failed"))?,
            recipient: recipient.value.clone().into_fixed_bytes().ok_or("can not recipient to bytes")?,
            payload: payload.value.clone().into_bytes().ok_or("can not payload to bytes")?
        })
    }
}

impl fmt::Display for Message {
    fn fmt(&self, m: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(m, "Message {{ 
            hash: {:x?}, 
            src_chain: {}, 
            src_sender:{:x?}, 
            nonce: {}, 
            dst_chain: {}, 
            recipient:{:x?}, 
            payload: {:x?} 
        }}", 
        hex::encode(&self.hash), self.src_chain, 
        hex::encode(&self.src_sender), self.nonce, 
        self.dst_chain, hex::encode(&self.recipient),
        hex::encode(&self.payload))
    }
}

#[derive(CandidType, Deserialize, Clone)]
pub enum Task {
    GetLogs,
    SendTx
}
