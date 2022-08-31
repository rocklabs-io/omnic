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

use crate::message::Message;

mod chain_info;
mod message;
mod chain_config;

thread_local! {
    static CHAINS: RefCell<HashMap<u32, ChainInfo>> = RefCell::new(HashMap::new());
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

#[update(name = "fetch_and_process_logs")]
#[candid_method(update, rename = "fetch_and_process_logs")]
async fn fetch_and_process_logs() -> Vec<Message> {

}

async fn fetch_and_process_logs() {

}


fn main() {}