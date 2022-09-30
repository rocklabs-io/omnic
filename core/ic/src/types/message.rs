
use candid::Deserialize;
use ic_web3::types::H256;
use crate::{utils::keccak256, OmnicError};
use crate::OmnicError::DecodeError;
use ic_web3::ethabi::{decode, encode, ParamType, Token, Error};

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Message {
    /// 4   SLIP-44 ID
    pub origin: u32,
    /// 32  Address in home convention
    pub sender: H256,
    /// 4   Count of all previous messages to destination
    pub nonce: u32,
    /// 4   SLIP-44 ID
    pub destination: u32,
    /// 32  Address in destination convention
    pub recipient: H256,
    /// 0+  Message contents
    pub body: Vec<u8>,
}

fn decode_body(data: &[u8]) -> Result<Vec<Token>, Error> {
    let types = vec![
        ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Uint(32), 
        ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Bytes
    ];
    decode(&types, data)
}

fn encode_body(msg: &Message) -> Vec<u8> {
    let tokens = [
        Token::Uint(msg.origin.into()),
        Token::FixedBytes(msg.sender.as_bytes().to_vec()),
        Token::Uint(msg.nonce.into()),
        Token::Uint(msg.destination.into()),
        Token::FixedBytes(msg.recipient.as_bytes().to_vec()),
        Token::Bytes(msg.body.clone())
    ];
    encode(&tokens)
}

impl Message {
    pub fn from_raw(raw_bytes: Vec<u8>) -> Result<Self, OmnicError> {
        let res = decode_body(&raw_bytes)?;
        let origin = res[0].clone().into_uint().ok_or(DecodeError("get origin failed".into()))?.as_u32();
        let sender_bytes = res[1].clone().into_fixed_bytes().ok_or(DecodeError("get sender failed".into()))?;
        let sender = H256::from_slice(&sender_bytes);
        let nonce = res[2].clone().into_uint().ok_or(DecodeError("get nonce failed".into()))?.as_u32();
        let destination = res[3].clone().into_uint().ok_or(DecodeError("get destination failed".into()))?.as_u32();
        let recipient_bytes = res[4].clone().into_fixed_bytes().ok_or(DecodeError("get recipient failed".into()))?;
        let recipient = H256::from_slice(&recipient_bytes);
        let body = res[5].clone().into_bytes().ok_or(DecodeError("get body failed".into()))?;

        Ok(Message {
            origin,
            sender,
            nonce,
            destination,
            recipient,
            body,
        })
    }
}

impl Message {
    /// Convert the message to a leaf
    pub fn to_leaf(&self) -> H256 {
        let raw = encode_body(&self);
        keccak256(&raw).into()
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Message {}->{}:{}",
            self.origin, self.destination, self.nonce,
        )
    }
}