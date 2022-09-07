

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

use omnic::home::Home;
use omnic::traits::HomeIndexer;
use omnic::chains::EVMChainIndexer;


thread_local! {
    static CHAINS: RefCell<HashMap<u32, Home<EVMChainIndexer>>> = RefCell::new(HashMap::new());
}

#[init]
#[candid_method(init)]
fn init() {
    // add goerli chain config
    // CHAINS.with(|chains| {
    //     let mut chains = chains.borrow_mut();
    //     // ledger.init_metadata(ic_cdk::caller(), args.clone());
    //     chains.insert(GOERLI_CHAIN_ID, ChainConfig {
    //         chain_id: GOERLI_CHAIN_ID,
    //         rpc_url: GOERLI_URL.clone().into(),
    //         omnic_addr:GOERLI_OMNIC_ADDR.clone().into(),
    //         omnic_start_block: 7468220,
    //         current_block: 7468220, 
    //         batch_size: 1000,
    //     });
    // });
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

// in heart_beat
async fn fetch_and_process_logs() {
    let mut chains = CHAINS.with(|chains| chains.borrow_mut());
    for (id, chain) in chains.iter_mut() {
        chain.sync_messages().await;
        // TODO: fetch root from proxy canister
        let proxy_root = H256::from_slice(&[0u8; 32]);
        let new_idx = match chain.update_tree(proxy_root) {
            Ok(v) => { v },
            Err(e) => { panic!("tree root match not found") },
        };
        while chain.processed_index < new_idx {
            // increase processed_index
            chain.increase_processed_index();
            // process message
            let proof = chain.generate_proof(chain.processed_index);
            // call proxy.process_message(message, proof)
        }
    }
    // CHAINS.with(|c| {
    //     let mut c = c.borrow_mut();
    //     c = chains;
    // });
}

// // in heart_beat
// async fn process_msgs() {
//     let chains = CHAINS.with(|chains| chains.borrow_mut());
//     for chain in chains {
//         chain.process_msgs();
//     }
// }

fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}