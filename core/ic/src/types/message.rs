
use candid::{CandidType, Deserialize};
use ic_web3::types::H256;
use crate::{utils::keccak256, OmnicError};
use crate::OmnicError::DecodeError;
use ic_web3::ethabi::{decode, encode, ParamType, Token, Error};


#[derive(PartialEq, Debug, Default, Clone, Deserialize, CandidType)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum MessageType {
    #[default]
    SYN = 0,
    ACK = 1,
    FAIL_ACK = 2
}

impl MessageType {
    fn from_u8(value: u8) -> MessageType {
        match value {
            1 => MessageType::SYN,
            2 => MessageType::ACK,
            3 => MessageType::FAIL_ACK,
            _ => panic!("Unknown value: {}", value),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Message {
    pub t: MessageType,
    /// 4   SLIP-44 ID
    pub origin: u32,
    /// 32  Address in home convention
    pub sender: H256,
    /// 4   Count of all previous messages to destination
    pub nonce: u64,
    /// 4   SLIP-44 ID
    pub destination: u32,
    /// 32  Address in destination convention
    pub recipient: H256,
    /// 0+  Message contents
    pub body: Vec<u8>,
}

pub fn decode_body(data: &[u8]) -> Result<Vec<Token>, Error> {
    let types = vec![
        ParamType::Uint(8), ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Uint(32), 
        ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Bytes
    ];
    decode(&types, data)
}

pub fn encode_body(msg: &Message) -> Vec<u8> {
    let tokens = [
        Token::Uint((msg.t.clone() as u8).into()),
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
        let msg_type = res[0].clone().into_uint().ok_or(DecodeError("get origin failed".into()))?.as_u32() as u8;
        let origin = res[1].clone().into_uint().ok_or(DecodeError("get origin failed".into()))?.as_u32();
        let sender_bytes = res[2].clone().into_fixed_bytes().ok_or(DecodeError("get sender failed".into()))?;
        let sender = H256::from_slice(&sender_bytes);
        let nonce = res[3].clone().into_uint().ok_or(DecodeError("get nonce failed".into()))?.as_u64();
        let destination = res[4].clone().into_uint().ok_or(DecodeError("get destination failed".into()))?.as_u32();
        let recipient_bytes = res[5].clone().into_fixed_bytes().ok_or(DecodeError("get recipient failed".into()))?;
        let recipient = H256::from_slice(&recipient_bytes);
        let body = res[6].clone().into_bytes().ok_or(DecodeError("get body failed".into()))?;

        Ok(Message {
            t: MessageType::from_u8(msg_type),
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
    // get message type
    pub fn get_msg_type(&self) -> MessageType {
        self.t.clone()
    }
}

impl From<MessageStable> for Message {
    fn from(s: MessageStable) -> Self {
        Self {
            t: MessageType::from_u8(s.t),
            origin: s.origin,
            sender: H256::from(s.sender),
            nonce: s.nonce,
            destination: s.destination,
            recipient: H256::from(s.recipient),
            body: s.body
        }
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Message {}->{}: sender {}, nonce: {}",
            self.origin, self.destination, self.sender, self.nonce,
        )
    }
}

#[derive(CandidType, Deserialize, Clone)]
pub struct MessageStable {
    pub t: u8,
    /// 4   SLIP-44 ID
    pub origin: u32,
    /// 32  Address in home convention
    pub sender: [u8;32],
    /// 4   Count of all previous messages to destination
    pub nonce: u64,
    /// 4   SLIP-44 ID
    pub destination: u32,
    /// 32  Address in destination convention
    pub recipient: [u8;32],
    /// 0+  Message contents
    pub body: Vec<u8>,
}

impl From<Message> for MessageStable {
    fn from(s: Message) -> Self {
        Self {
            t: s.t as u8,
            origin: s.origin,
            sender: s.sender.to_fixed_bytes(),
            nonce: s.nonce,
            destination: s.destination,
            recipient: s.recipient.to_fixed_bytes(),
            body: s.body
        }
    }
}

