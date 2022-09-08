

/*
omnic relayer canister:
    fetch_and_process_logs: 
        EnqueueMessage: insert to incoming msgs, add to merkle tree
        ProcessMessage: remove confirmed outgoing msgs
    process_msgs: traverse incoming_msgs for each chain, 
        generate corresponding outgoing msg and insert into corresponding chain's outgoing_msg queue
    send_msgs: traverse outgoing_msgs for each chain,
        for ic, call recipient canister.handle_message, remove the message from outgoing_msg queue and send to history storage
        for other evm chains, batch send txs, but not remove the msg from queue yet, wait for ProcessMessage event (fetch_and_process_logs)
*/

// fetch_root can be done in heart_beat, others can be triggered by offchain worker

use std::collections::HashMap;
use std::cell::RefCell;
use ic_cdk::api::{call::CallResult, canister_balance};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Int, Nat};
use ic_cdk::export::Principal;
use ic_web3::types::H256;

use omnic::{OmnicError, RawMessage};
use omnic::home::Home;
use omnic::traits::HomeIndexer;
use omnic::chains::{EVMChainIndexer, IndexerConfig};

const PROXY: &str = "";
const GOERLI_URL: &str = "https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm";
const GOERLI_CHAIN_ID: u32 = 5;
const GOERLI_OMNIC_ADDR: &str = "7E58Df2620ADDa3BA6FF6Aca989343D11807450E";
const EVENT_ENQUEUE_MSG: &str = "84ec73a8411e8551ef1faab6c2277072efce9d5e4cc2ae5a218520dcdd7a377c";

thread_local! {
    static CHAINS: RefCell<HashMap<u32, Home>> = RefCell::new(HashMap::new());
}

#[init]
#[candid_method(init)]
fn init() {
    // add goerli chain config
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        let indexer_config = IndexerConfig {
            chain_id: GOERLI_CHAIN_ID,
            rpc_url: GOERLI_URL.to_string(),
            omnic_addr: GOERLI_OMNIC_ADDR.to_string(),
        };
        let start_block = 7552168;
        let batch_size = 1000;
        chains.insert(GOERLI_CHAIN_ID, Home::new(indexer_config, start_block, batch_size));
    });
}

// #[update(name = "add_chain_config")]
// #[candid_method(update, rename = "add_chain_config")]
// async fn add_chain_config(config: ChainConfig) -> Result<bool, String> {
//     CHAINS.with(|chains| {
//         let mut chains = chains.borrow_mut();
//         if chains.contains_key(&config.chain_id) {
//              Err("chain exists".into())
//         } else {
//             chains.insert(config.chain_id, ChainInfo::new(config));
//             Ok(true)
//         }
//     })
// }

#[update(name = "sync")]
#[candid_method(update, rename = "sync")]
async fn sync() -> Result<bool, String> {
    process().await;
    CHAINS.with(|chains| {
        let chains = chains.borrow();
        for (id, chain) in chains.iter() {
            ic_cdk::println!("chain: {}", id);
            ic_cdk::println!("db: {:?}", chain.db);
        }
    });
    Ok(true)
}

fn insert_messages(chain_id: u32, new_block: u32, msgs: Vec<RawMessage>) -> Result<(), OmnicError> {
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        match chains.get_mut(&chain_id) {
            Some(c) => {
                match c.db.store_messages(&msgs) {
                    Ok(_) => { c.current_block = new_block; Ok(()) },
                    Err(e) => { Err(e) },
                }
            },
            None => {
                Err(OmnicError::Other("chain id not found".into()))
            },
        }
    })
}

async fn sync_home_messages(chain_id: u32, home: &Home) -> Result<(), OmnicError> {
    // fetch messages
    let indexer = EVMChainIndexer::new(home.indexer_config.clone())?;
    let block_number = indexer.get_block_number().await?;
    let to = if block_number < home.current_block + home.batch_size {
        block_number
    } else {
        home.current_block + home.batch_size
    };
    let msgs = indexer.fetch_sorted_messages(home.current_block, to).await?;
    // insert messages to home db
    insert_messages(chain_id, to, msgs)?;
    Ok(())
}

async fn update_tree_and_proof(chain_id: u32) -> Result<(), OmnicError> {
    // TODO: fetch root from proxy canister
    // let root = ic_cdk::api::call(Principal::from_slice(PROXY), "get_latest_root", (chain_id, )).await;
    let proxy_root = H256::from_slice(&[0u8; 32]);
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        match chains.get_mut(&chain_id) {
            Some(c) => {
                let mut idx = c.index;
                if let Ok(new_idx) = c.update_tree(proxy_root) {
                    while idx < new_idx {
                        c.generate_and_store_proof(idx);
                        idx += 1;
                    }
                };
                Ok(())
            },
            None => {
                Err(OmnicError::Other("chain id not found".into()))
            },
        }
    })
}

// TODO: complete implementation
// async fn dispatch_messages(chain_id: u32, home: &Home) {
//     let res = CHAINS.with(|chains| {
//         let chains = chains.borrow();
//         match chains.get(chain_id) {
//             Some(c) => {
//                 c.fresh_proven_messages_with_proof()
//             },
//             None => {},
//         }
//     })

//     for (msg, proof) in res {
//         // call proxy.process_message(msg, proof)
//     }

//     // increase processed_index for this chain
//     CHAINS.with(|chains| {
//         let mut chains = chains.borrow_mut();
//         match chains.get(chain_id) {
//             Some(c) => {
//                 c.set_processed_index(c.index);
//             },
//             None => {},
//         }
//     })
// }

// in heart_beat
async fn process() {
    let chains = CHAINS.with(|chains| chains.borrow().clone());
    for (id, chain) in chains.iter() {
        // fetch messages
        match sync_home_messages(*id, chain).await {
            Ok(_) => {},
            Err(_) => { continue },
        }
        // fetch root from proxy canister, update tree to catch up, and generate proofs for messages in between
        // match update_tree_and_proof(*id).await {
        //     Ok(_) => {},
        //     Err(_) => { continue },
        // }
        // send proven messages to proxy
        // match dispatch_messages(id).await {
        //
        // }
    }
}

fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}