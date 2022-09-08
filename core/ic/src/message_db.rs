// message storage

use std::collections::{VecDeque, HashMap};
use ic_web3::types::H256;

use accumulator::{Proof, TREE_DEPTH};

use crate::types::{Message, RawMessage};
use crate::error::OmnicError;

#[derive(Clone, Debug)]
pub struct MessageDB {
    pub msgs: VecDeque<RawMessage>,
    pub proofs: HashMap<u32, Proof<TREE_DEPTH>>,
    pub latest_leaf_index: u32,
}

impl MessageDB {
    pub fn new() -> Self {
        MessageDB {
            msgs: VecDeque::new(),
            proofs: HashMap::new(),
            latest_leaf_index: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.latest_leaf_index == 0
    }

    pub fn store_messages(&mut self, messages: &[RawMessage]) -> Result<(), OmnicError> {
        for message in messages {
            self.store_latest_message(message)?;
        }
        Ok(())
    }

    pub fn store_latest_message(&mut self, message: &RawMessage) -> Result<(), OmnicError> {
        if self.latest_leaf_index == message.leaf_index - 1 {
            self.latest_leaf_index += 1;
            self.msgs.push_back(message.clone());
            Ok(())
        } else {
            Err(OmnicError::DBError(
                format!("message.leaf_index {} != latest_leaf_index {} + 1", message.leaf_index, self.latest_leaf_index)
            ))
        }
    }

    pub fn store_proof(&mut self, leaf_index: u32, proof: &Proof<TREE_DEPTH>) {
        self.proofs.insert(leaf_index, proof.clone());
    }

    pub fn proof_by_leaf_index(&self, leaf_index: u32) -> Result<Proof<TREE_DEPTH>, OmnicError> {
        match self.proofs.get(&leaf_index) {
            Some(v) => { Ok(*v) },
            None => { 
                Err(OmnicError::DBError(
                    format!("proof not found for leaf index {}", leaf_index)
                ))
            },
        }
    }

    pub fn delete_proof(&mut self, leaf_index: u32) {
        self.proofs.remove(&leaf_index);
    }

    pub fn message_by_leaf_index(&self, index: u32) -> Result<RawMessage, OmnicError> {
        let res = self.msgs.get(index as usize).ok_or(OmnicError::DBError(
            format!("message at leaf index {} not found", index)
        ))?;
        Ok(res.clone())
    }

    pub fn message_and_proof_by_leaf_index(&self, index: u32) -> Result<(RawMessage, Proof<TREE_DEPTH>), OmnicError> {
        Ok(
            (
                self.message_by_leaf_index(index)?,
                self.proof_by_leaf_index(index)?
            )
        )
    }

    pub fn leaf_by_leaf_index(&self, index: u32) -> Result<H256, OmnicError> {
        let res = self.msgs.get(index as usize).ok_or(OmnicError::DBError(
            format!("message at leaf index {} not found", index)
        ))?;
        Ok(res.hash)
    }
}