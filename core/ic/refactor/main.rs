/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
    fetch_and_process_logs: 
        EnqueueMessage: insert to incoming msgs
        ProcessMessage: remove confirmed outgoing msgs
    process_msgs: traverse incoming_msgs for each chain, 
        generate corresponding outgoing msg and insert into corresponding chain's outgoing_msg queue
    send_msgs: traverse outgoing_msgs for each chain,
        for ic, call recipient canister.handle_message, remove the message from outgoing_msg queue and send to history storage
        for other evm chains, batch send txs, but not remove the msg from queue yet, wait for ProcessMessage event (fetch_and_process_logs)
*/

// fetch_root can be done in heart_beat, others can be triggered by offchain worker

#[update(name = "fetch_and_process_logs")]
#[candid_method(update, rename = "fetch_and_process_logs")]
async fn fetch_and_process_logs() -> Vec<Message> {

}


