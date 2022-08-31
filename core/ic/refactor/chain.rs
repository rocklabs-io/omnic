
/*
    chain struct
    maintain a incoming message queue & a outgoing message queue
*/

use std::collections::{HashMap, VecDeque};
use crate::chain_config::ChainConfig;
use crate::Message;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct Chain {
    pub config: ChainConfig,
    // TODO what's Byte32?   [u8; 32]?
    pub roots: HashMap<Bytes32, u64>, // root hash -> confirm time
    pub incoming_msgs: VecDeque<Message>,
    pub outgoing_msgs: VecDeque<Message>,
    pub confirming: VecDeque<Message>,
}

impl Chain {
    pub fn set_current_block(&mut self, v: u64) {
        self.config.set_current_block(v);
    }

    pub fn set_batch_size(&mut self, v: u64) {
        self.config.set_batch_size(v);
    }

    fn insert_incoming(&mut self, msg: Message) {
        self.incoming_msgs.push_back(msg);
    }

    fn insert_outgoing(&mut self, msg: Message) {
        self.outgoing_msgs.push_back(msg);
    }

    fn verify(&self, msg: &Message) -> bool {
        
    }

    // TODO get messages from each queue, batch or one-by-one?

    // fn put_root()
}