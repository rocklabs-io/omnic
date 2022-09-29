use ic_web3::types::{H256, Log};
use ic_web3::ethabi::{decode, Event, EventParam, ParamType, RawLog, Token, Error};
use std::convert::TryFrom;
use crate::{types::Message, OmnicError};
use crate::OmnicError::LogDecodeError;
use crate::traits::encode::Decode;

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

fn _decode_body(data: &[u8]) -> Result<Vec<Token>, Error> {
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

        // ic_cdk::println!("log: {:?}", log.clone());

        let params = vec![
            EventParam { name: "messageHash".to_string(), kind: ParamType::FixedBytes(32), indexed: true },
            EventParam { name: "leafIndex".to_string(), kind: ParamType::Uint(256), indexed: true },
            EventParam { name: "dstChainId".to_string(), kind: ParamType::Uint(32), indexed: true },
            EventParam { name: "nonce".to_string(), kind: ParamType::Uint(32), indexed: false },
            EventParam { name: "message".to_string(), kind: ParamType::Bytes, indexed: false },
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

        // ic_cdk::println!("res: {:?}", res.clone());
        
        let msg_hash = res.params.iter().find(|p| p.name == "messageHash").ok_or(LogDecodeError("missing messgaHash".into()))?;
        let leaf_index = res.params.iter().find(|p| p.name == "leafIndex").ok_or(LogDecodeError("missing leafIndex".into()))?;
        let _dst_chain = res.params.iter().find(|p| p.name == "dstChainId").ok_or(LogDecodeError("missing dstChainId".into()))?;
        let _dst_nonce = res.params.iter().find(|p| p.name == "nonce").ok_or(LogDecodeError("missing nonce".into()))?;
        let message = res.params.iter().find(|p| p.name == "message").ok_or(LogDecodeError("missing message".into()))?;

        // ic_cdk::println!("msg hash: {:?}", H256::from_slice(&msg_hash.value.clone().into_fixed_bytes().unwrap()));

        let msg = Message::read_from(
            &mut message.value.clone().into_bytes()
                .ok_or(LogDecodeError("cannot convert message to bytes".into()))?
                .as_slice()
        )?;

        let m = RawMessage {
            hash: H256::from_slice(&msg_hash.value.clone().into_fixed_bytes().ok_or(LogDecodeError("hash decode error".into()))?),
            leaf_index: leaf_index.value.clone().into_uint().ok_or(LogDecodeError("cannot convert uint".into()))?.as_u32(),
            message: msg,
        };
        // ic_cdk::println!("to_leaf: {:?}", m.to_leaf());
        if m.to_leaf() != m.hash {
            Err(LogDecodeError("hash mismatch".into()))
        } else {
            Ok(m)
        }
    }
}