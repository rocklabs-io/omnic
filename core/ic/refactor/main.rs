/*
    fetch_root
    fetch_and_process_logs: insert to incoming msgs & remove confirmed outgoing msgs
    send_txs
*/

#[update(name = "fetch_root")]
#[candid_method(update, rename = "fetch_root")]
async fn get() -> Vec<Message> {
    traverse_chains().await
}


