/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
*/

use crate::accumulator::Proof;
use crate::message::Message;

mod chain_roots;
mod message;
mod chain_config;

const let OPTIMISTIC_DELAY: u64 = 1800; // 30 mins

thread_local! {
    static CHAINS: RefCell<HashMap<u32, ChainRoots>> = RefCell::new(HashMap::new());
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

// relayer canister call this to check if a message is valid before process_message
#[query(name = "is_valid")]
#[candid_method(query, rename = "is_valid")]
async fn is_valid(proof: Proof<32>, message: Message) -> Result<bool, String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    //     if message.need_verify is false, we only check if root exist in the hashmap
    //     if message.need_verify is true, we additionally check root.confirm_at <= ic_cdk::api::time()
}

#[update(name = "process_message")]
#[candid_method(update, rename = "process_message")]
async fn process_message(proof: Proof<32>, message: Message) -> Result<bool, String> {
    // verify message proof: use proof, message to calculate the merkle root, 
    //     if message.need_verify is false, we only check if root exist in the hashmap
    //     if message.need_verify is true, we additionally check root.confirm_at <= ic_cdk::api::time()
    // if valid, call dest canister.handleMessage or send tx to dest chain
    // if invalid, return error
}

async fn fetch_root(chain: &ChainRoots) -> Result<H256, String> {
    // query omnic contract.getLatestRoot, 
    // fetch from multiple rpc providers and aggregrate results, should be exact match
}

// this is done in heart_beat
async fn fetch_roots() -> Result<bool, String> {
    let chains = CHAINS.with(|chains| {
        chains.borrow().clone()
    });
    for (id, chain) in chains.into_iter() {
        let root = if let Ok(v) = fetch_root(&chain).await {
            v
        } else {
            ic_cdk::println!("fetch root failed for: {:?}", &chain.config);
            continue
        };
        let confirm_at = ic_cdk::api::time() + OPTIMISTIC_DELAY;
        CHAINS.with(|chains| {
            let mut chains = chains.borrow_mut();
            match chains.get(id) {
                Some(v) => v.insert_root(root, confirm_at);
                None => unreachable!();
            }
        });
    }
    Ok(true)
}


fn main() {}