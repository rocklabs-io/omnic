
/*
    used in relayer
    fetch crosschain messages from chain, maintain a merkle tree for the corresponding chain messages
    generate merkle proof and send message with proof to omnic proxy canister to process the message
*/

use std::collections::{HashMap, VecDeque};
use crate::chain_config::ChainConfig;
use crate::Message;
use crate::accumulator::tree;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ChainInfo {
    pub config: ChainConfig,
    pub tree: tree::Tree<32>,
    pub incoming: VecDeque<Message>, // incoming messages
    pub confirming: VecDeque<Message>, // processed messages, wait confirmation
}

impl ChainInfo {
    pub fn set_current_block(&mut self, v: u64) {
        self.config.set_current_block(v);
    }

    pub fn set_batch_size(&mut self, v: u64) {
        self.config.set_batch_size(v);
    }

    fn insert_incoming(&mut self, msg: Message) {
        self.incoming_msgs.push_back(msg);
        // insert msg hash to merkle tree
    }

    // TODO get messages from each queue, batch or one-by-one?

    // fn put_root()
}