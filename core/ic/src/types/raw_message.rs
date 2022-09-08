use ic_web3::types::{H256, Log};
use ic_web3::ethabi::{decode, Event, EventParam, ParamType, RawLog, Token, Error};
use std::convert::{TryFrom, TryInto};
use crate::{types::Message, OmnicError};
use crate::OmnicError::LogDecodeError;

#[derive(Debug, Default, Clone)]
pub struct RawMessage {
    /// msg hash
    pub hash: H256,
    /// The index at which the message is committed
    pub leaf_index: u32,
    /// The fully detailed message that was committed
    pub message: Message,
}

impl RawMessage {
    /// Return the leaf associated with the message
    pub fn to_leaf(&self) -> H256 {
        self.message.to_leaf()
    }
}

impl AsRef<Message> for RawMessage {
    fn as_ref(&self) -> &Message {
        &self.message
    }
}

fn decode_body(data: &[u8]) -> Result<Vec<Token>, Error> {
    let types = vec![
        ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Uint(32), 
        ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Bool,
        ParamType::Bytes
    ];
    decode(&types, data)
}

impl TryFrom<Log> for RawMessage {
    type Error = OmnicError;

    fn try_from(log: Log) -> Result<Self, Self::Error> {

        ic_cdk::println!("log: {:?}", log.clone());

        let params = vec![
            EventParam { name: "messageHash".to_string(), kind: ParamType::FixedBytes(32), indexed: true },
            EventParam { name: "leafIndex".to_string(), kind: ParamType::Uint(256), indexed: true },
            EventParam { name: "dstChainId".to_string(), kind: ParamType::Uint(32), indexed: true },
            EventParam { name: "nonce".to_string(), kind: ParamType::Uint(32), indexed: false },
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
        })?;

        ic_cdk::println!("res: {:?}", res.clone());
        
        let msg_hash = res.params.iter().find(|p| p.name == "messageHash").ok_or(LogDecodeError("missing messgaHash".into()))?;
        let leaf_index = res.params.iter().find(|p| p.name == "leafIndex").ok_or(LogDecodeError("missing leafIndex".into()))?;
        let dst_chain = res.params.iter().find(|p| p.name == "dstChainId").ok_or(LogDecodeError("missing dstChainId".into()))?;
        let dst_nonce = res.params.iter().find(|p| p.name == "nonce").ok_or(LogDecodeError("missing nonce".into()))?;
        let data = res.params.iter().find(|p| p.name == "payload").ok_or(LogDecodeError("missing payload".into()))?;

        ic_cdk::println!("msg hash: {:?}", H256::from_slice(&msg_hash.value.clone().into_fixed_bytes().unwrap()));

        // decode data to get message fields
        let decoded = decode_body(&data.value.clone().into_bytes().ok_or(LogDecodeError("cannot convert data to bytes".into()))?)?;
        let src_chain = decoded[0].clone().into_uint().ok_or(LogDecodeError("cannot convert src_chain to U256".into()))?.as_u32();
        let src_sender = decoded[1].clone().into_fixed_bytes().ok_or(LogDecodeError("cannot convert src_sender to bytes".into()))?;
        let recipient = decoded[4].clone().into_fixed_bytes().ok_or(LogDecodeError("cannot convert recipient to bytes".into()))?;
        let wait_optimistic = decoded[5].clone().into_bool().ok_or(LogDecodeError("cannot convert bool".into()))?;
        let payload = decoded[6].clone().into_bytes().ok_or(LogDecodeError("cannot convert payload to bytes".into()))?;


        let m = RawMessage {
            hash: H256::from_slice(&msg_hash.value.clone().into_fixed_bytes().ok_or(LogDecodeError("hash decode error".into()))?),
            leaf_index: leaf_index.value.clone().into_uint().ok_or(LogDecodeError("cannot convert uint".into()))?.as_u32(),
            message: Message {
                origin: src_chain,
                sender: H256::from_slice(&src_sender),
                nonce: dst_nonce.value.clone().into_uint().ok_or(LogDecodeError("can not convert nonce to U256".into()))?.as_u32(),
                destination: dst_chain.value.clone().into_uint().ok_or(LogDecodeError("can not convert dst_chain to U256".into()))?.as_u32(),
                recipient: H256::from_slice(&recipient),
                wait_optimistic,
                body: payload,
            }
        };
        ic_cdk::println!("to_leaf: {:?}", m.to_leaf());
        if m.to_leaf() != m.hash {
            Err(LogDecodeError("hash mismatch".into()))
        } else {
            Ok(m)
        }
    }
}