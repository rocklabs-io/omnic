use std::fmt;
use std::convert::TryInto;
use ic_web3::ethabi::{decode, Event, EventParam, ParamType, RawLog, Token};
use ic_web3::types::{Log, H256, U256, SignedTransaction};
use ic_cdk::export::candid::{CandidType, Deserialize};

#[derive(Clone, Debug, Deserialize)]
pub struct Message {
    pub log: Log, // origin log for this message

    pub hash: H256,
    pub leaf_index: U256,
    // message body
    pub src_chain: u32,
    pub src_sender: Vec<u8>,
    pub nonce: u32,
    pub dst_chain: u32,
    pub recipient: Vec<u8>,
    pub payload: Vec<u8>,
    pub wait_optimistic: bool,

    #[serde(skip)]
    pub verified: bool, // optimistically verified
    #[serde(skip)]
    pub outgoing_tx: Option<SignedTransaction>, // if dst = ic, no need this
    #[serde(skip)]
    pub outgoing_tx_confirmed: bool,
    #[serde(skip)]
    pub processed_log: Option<Log>, // log emitted after this msg is processed on the destination chain
}