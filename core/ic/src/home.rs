use accumulator::{tree::Tree, Proof, Merkle, TREE_DEPTH};
use ic_web3::types::H256;

use crate::{RawMessage, MessageDB, HomeIndexer, error::OmnicError, chains::config::IndexerConfig};

#[derive(Clone)]
pub struct Home {
    pub indexer_config: IndexerConfig,
    pub db: MessageDB,
    tree: Tree<TREE_DEPTH>,
    pub index: u32, // latest message index that has been inserted into the tree
    pub processed_index: u32,
    pub start_block: u32,
    pub current_block: u32,
    pub batch_size: u32,
}

impl Home {

    pub fn new(indexer_config: IndexerConfig, start_block: u32, batch_size: u32) -> Self {
        Home {
            indexer_config,
            db: MessageDB::new(),
            tree: Tree::<TREE_DEPTH>::default(),
            index: 0,
            processed_index: 0,
            start_block,
            current_block: start_block,
            batch_size,
        }
    }

    /// new_root: fetched from proxy canister, update local tree to catch up
    /// can process messages between (self.processed_index, idx] if update_tree success
    pub fn update_tree(&mut self, new_root: H256) -> Result<u32, OmnicError> {
        let mut idx = self.index;
        while self.tree.root() != new_root {
            self.tree.ingest(self.db.leaf_by_leaf_index(idx)?);
        }
        if self.tree.root() == new_root {
            Ok(idx)
        } else {
            Err(OmnicError::HomeError("new index not found".into()))
        }
    }

    pub fn generate_proof(&self, index: u32) -> Result<Proof<TREE_DEPTH>, OmnicError> {
        Ok(self.tree.prove(index as usize)?)
    }

    pub fn generate_and_store_proof(&mut self, index: u32) -> Result<(), OmnicError> {
        let proof = self.tree.prove(index as usize)?;
        self.db.store_proof(index, &proof);
        Ok(())
    }

    // fetch proven messages, messages between (self.processed_index, self.index]
    pub fn fresh_proven_messages_with_proof(&self) -> Result<Vec<(RawMessage, Proof<TREE_DEPTH>)>, OmnicError> {
        let mut res: Vec<(RawMessage, Proof<TREE_DEPTH>)> = Vec::new();
        for i in (self.processed_index + 1)..=self.index {
            let item = self.db.message_and_proof_by_leaf_index(i)?;
            res.push(item);
        }
        Ok(res)
    }

    pub fn increase_processed_index(&mut self) {
        self.processed_index += 1;
        self.db.delete_proof(self.processed_index);
    }

    pub fn set_processed_index(&mut self, v: u32) {
        while self.processed_index < v {
            self.increase_processed_index();
        }
    }
}

