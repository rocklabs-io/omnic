use accumulator::{tree::Tree, Proof, Merkle, TREE_DEPTH};
use ic_web3::types::H256;

use crate::{MessageDB, HomeIndexer, error::OmnicError};

pub struct Home<T: HomeIndexer> {
    indexer: T,
    db: MessageDB,
    tree: Tree<TREE_DEPTH>,
    index: u32, // latest message index that has been inserted into the tree
    processed_index: u32,
    start_block: u32,
    current_block: u32,
    batch_size: u32,
}

impl<T> Home<T> where T: HomeIndexer {

    pub fn new(indexer: T, start_block: u32, batch_size: u32) -> Self {
        Home {
            indexer,
            db: MessageDB::new(),
            tree: Tree::<TREE_DEPTH>::default(),
            index: 0,
            processed_index: 0,
            start_block,
            current_block: start_block,
            batch_size,
        }
    }

    pub async fn sync_messages(&mut self) -> Result<(), OmnicError> {
        let block_number = self.indexer.get_block_number().await?;
        let to = if block_number < self.current_block + self.batch_size {
            block_number
        } else {
            self.current_block + self.batch_size
        };
        let msgs = self.indexer.fetch_sorted_messages(self.current_block, to).await?;
        self.db.store_messages(&msgs)?;
        Ok(())
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
}

