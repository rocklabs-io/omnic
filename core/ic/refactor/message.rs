use std::fmt;
use ethabi::decode;
use ic_web3::ethabi::{Event, EventParam, ParamType, RawLog};
use ic_web3::types::{Log, U256};

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Message {
    pub log: Log, // origin log for this message

    pub hash: Vec<u8>,
    pub leaf_index: U256,
    // message body
    pub src_chain: u32,
    pub src_sender: Vec<u8>,
    pub nonce: u32,
    pub dst_chain: u32,
    pub recipient: Vec<u8>,
    pub payload: Vec<u8>,
    pub wait_optimistic: bool,

    pub verified: bool, // optimistically verified
    pub outgoing_tx: Option<SignedTransaction>, // if dst = ic, no need this
    pub outgoing_tx_confirmed: bool,
    pub processed_log: Option<Log>, // log emitted after this msg is processed on the destination chain
}

fn decode_body(data: &[u8]) -> Result<Vec<Token>, String> {
    let types = vec![
        ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Uint(32), 
        ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Bool,
        ParamType::Bytes
    ];
    decode(&types, data).map_err(|e| format!("payload decode error"))?
}

impl Message {

    pub fn set_verified(&mut self) {
        self.verified = true;
    }

    pub fn set_outgoing_tx(&mut self, tx: SignedTransaction) {
        self.outgoing_tx = Some(tx);
    }

    pub fn set_outgoing_tx_status(&mut self, confirmed: bool) {
        self.outgoing_tx_confirmed = confirmed;
    }

    pub fn set_processed_log(&mut self, log: &Log) {
        self.processed_log = log.clone();
    }

    pub fn from_log(log: &Log) -> Result<Message, String> {
        let params = vec![
            EventParam { name: "messageHash".to_string(), kind: ParamType::FixedBytes(32), indexed: true },
            EventParam { name: "leafIndex".to_string(), kind: ParamType::Uint(256), indexed: true },
            EventParam { name: "dstChainId".to_string(), kind: ParamType::Uint(32), indexed: true },
            EventParam { name: "nonce".to_string(), kind: ParamType::FixedBytes(32), indexed: false },
            EventParam { name: "payload".to_string(), kind: ParamType::Bytes, indexed: false },
        ];

        let event = Event {
            name: "SendMessage".to_string(),
            inputs: params,
            anonymous: false
        };
        let res = event.parse_log(RawLog {
            topics: log.topics.clone(),
            data: log.data.clone().0
        }).map_err(|e| format!("ethabi parse_log failed: {}", e))?;
        
        let msg_hash = res.params.iter().find(|p| p.name == "messageHash").ok_or("missing messgaHash".to_string())?;
        let leaf_index = res.params.iter().find(|p| p.name == "leafIndex").ok_or("missing leafIndex".to_string())?;
        let dst_chain = res.params.iter().find(|p| p.name == "dstChainId").ok_or("missing dstChainId".to_string())?;
        let dst_nonce = res.params.iter().find(|p| p.name == "nonce").ok_or("missing nonce".to_string())?;
        let data = res.params.iter().find(|p| p.name == "payload").ok_or("missing payload".to_string())?;
        // ic_cdk::println!("event: {:?}", res);
        // ic_cdk::println!("msg_hash: {:?}", msg_hash.value.clone());

        // decode data to get message body fields
        let decoded = decode_body(&data.into_bytes())?;
        let src_chain = decoded[0].into_uint().ok_or("can not convert src_chain to U256")?.try_into().map_err(|_| format!("convert U256 to u32 failed"))?;
        let src_sender = decoded[1].into_fixed_bytes().ok_or("can not convert src_sender to bytes")?;
        let recipient = decoded[4].into_fixed_bytes().ok_or("can not convert recipient to bytes")?;
        let wait_optimistic = decoded[5].into_bool().ok_or("can not convert bool")?;
        let payload = decoded[6].into_bytes().ok_or("cannot convert payload to bytes")?;

        Ok(Message {
            log: log.clone(),
            hash: msg_hash.value.clone().into_fixed_bytes().ok_or("can not convert hash to bytes")?,
            leaf_index: leaf_index.value.clone(),

            src_chain: src_chain,
            src_sender: src_sender.
            nonce: dst_nonce.value.clone().into_uint().ok_or("can not convert nonce to U256")?.try_into().map_err(|_| format!("convert U256 to u32 failed"))?,
            dst_chain: dst_chain.value.clone().into_uint().ok_or("can not convert dst_chain to U256")?.try_into().map_err(|_| format!("convert U256 to u32 failed"))?,
            recipient: recipient,
            wait_optimistic: wait_optimistic,
            payload: payload,

            // TODO new add field setting
            verified: false,
            outgoing_tx: None,
            outgoing_tx_confirmed: false,
            processed_log: None
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