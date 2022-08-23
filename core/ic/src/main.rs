use std::str::FromStr;
use std::cell::RefCell;
use std::str;
use std::collections::HashMap;

use omnic::types::{InitArgs, ChainConfig, Message, Task};

use ic_cdk::api::{call::CallResult, canister_balance};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Int, Nat};
use ic_cdk::export::Principal;

use ic_cron::task_scheduler::TaskScheduler;
use ic_cron::types::Iterations;

use ic_web3::transports::ICHttp;
use ic_web3::Web3;
use ic_web3::ic::{get_eth_addr, KeyInfo};
use ic_web3::{
    contract::{Contract, Options},
    ethabi::ethereum_types::{U64, U256, H256, H160},
    ethabi::{Event, EventParam, ParamType, Log as ABILog, RawLog},
    types::{Address, TransactionParameters, BlockId, BlockNumber, FilterBuilder, Log},
};

// goerli testnet rpc url
// const URL: &str = "https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd";
const GOERLI_URL: &str = "https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm";
const GOERLI_CHAIN_ID: u32 = 5;
const GOERLI_OMNIC_ADDR: &str = "C3bfE8E4f99C13eb8f92C944a89C71E7be178A6F";
const EVENT_ENQUEUE_MSG: &str = "b9bede5465bf01e11c8b770ae40cbae2a14ace602a176c8ea626c9fb38a90bd8";

const KEY_NAME: &str = "dfx_test_key";
// const TOKEN_ABI: &[u8] = include_bytes!("../src/contract/res/token.json");

type Result<T, E> = std::result::Result<T, E>;

ic_cron::implement_cron!();

#[derive(CandidType, Deserialize, Default)]
struct State {
    chains: HashMap<u32, ChainConfig>, // supported chains
    cron_state: Option<TaskScheduler>,
}

thread_local! {
    static CHAINS: RefCell<HashMap<u32, ChainConfig>> = RefCell::new(HashMap::new());
    // / message queue to be processed
    // static MSGS: RefCell<>;
    // / processed messages
    // static PROCESSED_MSGS: RefCell<>;
    // / outgoing tx queue
    // static TXS: RefCell<>;
}

#[init]
#[candid_method(init)]
fn init() {
    cron_enqueue(
        Task::GetLogs,
        ic_cron::types::SchedulingOptions {
            delay_nano: 10_000_000_000,
            interval_nano: 10_000_000_000, // every 10 seconds
            iterations: Iterations::Infinite,
        },
    ).unwrap();
    // add goerli chain config
    CHAINS.with(|chains| {
        let mut chains = chains.borrow_mut();
        // ledger.init_metadata(ic_cdk::caller(), args.clone());
        chains.insert(GOERLI_CHAIN_ID, ChainConfig {
            chain_id: GOERLI_CHAIN_ID,
            rpc_url: GOERLI_URL.clone().into(),
            omnic_addr:GOERLI_OMNIC_ADDR.clone().into(),
            omnic_start_block: 7426080,
            current_block: 7426080, 
            batch_size: 1000,
        });
    });
}

#[update(name = "get_logs")]
#[candid_method(update, rename = "get_logs")]
async fn get() -> Result<Vec<Message>, String> {
    let chains = CHAINS.with(|chains| {
        chains.borrow().clone()
    });
    ic_cdk::println!("chains: {:?}", chains);
    ic_cdk::println!("getting messages from chains now...");
    let mut res = Vec::new();
    for (id, chain) in chains.into_iter() {
        let msgs = get_chain_msgs(&chain).await.unwrap();
        ic_cdk::println!("msgs: {:?}", &msgs);
        res.extend(msgs);
    }
    Ok(res)
}

// process messages
// async fn process_msgs() -> Result<Bool, String> {

// }

async fn get_chain_msgs(chain: &ChainConfig) -> Result<Vec<Message>, String> {
    let filter = FilterBuilder::default()
        .address(vec![H160::from_str(&chain.omnic_addr).unwrap()])
        .topics(
            Some(vec![H256::from_str(EVENT_ENQUEUE_MSG).unwrap()]),
            None,
            None,
            None,
        )
        .from_block(BlockNumber::Number(chain.current_block.into()))
        .to_block(BlockNumber::Latest) // todo: min(chain.current_block + chain.batch_size, block_height)
        .build();
    let w3 = match ICHttp::new(&chain.rpc_url, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { panic!() },
    };
    let logs = w3.eth().logs(filter).await.unwrap();
    Ok(logs.iter().map(|log| Message::from_log(&log).unwrap()).collect())
}

/*
heartbeat tasks:
    GetLogs: fetch logs from supported chains and enqueue messages
    ProcessMsgs: process messages from msg queue, 
        if destination is IC canister, call canister.handleMessage
        if destination is EVM chain, construct & sign the tx and enqueue to tx queue
    SendTxs: send pending txs to external EVM chains
*/
#[heartbeat]
fn heartbeat() {
    for task in cron_ready_tasks() {
        let kind = task.get_payload::<Task>().expect("Serialization error");
        match kind {
            Task::GetLogs => todo!(),
            SendTx => todo!()
        }
    }
}

fn get_time() -> u64 {
    ic_cdk::api::time() / 1000000000
}

// fn is_custodian() -> bool {
    
// }

// fn is_operator(sender: &Principal) -> bool {
    
// }

fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}