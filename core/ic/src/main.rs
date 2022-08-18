use std::str::FromStr;
use std::cell::RefCell;

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
    types::{Address, TransactionParameters, BlockId, BlockNumber, FilterBuilder},
};

// goerli testnet rpc url
const URL: &str = "https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd";
const CHAIN_ID: u64 = 5;
const ETH_OMNIC_ADDR: &str = "0fA355bEEA41d190CAE64F24a58F70ff2912D7df";
const EVENT_ENQUEUE_MSG: &str = "49855fe1b89449bbbf62ad50dd54754b7834260e96c7986a103cbcb95883353c";

const KEY_NAME: &str = "dfx_test_key";
// const TOKEN_ABI: &[u8] = include_bytes!("../src/contract/res/token.json");

type Result<T, E> = std::result::Result<T, E>;

ic_cron::implement_cron!();

#[derive(CandidType, Deserialize, Default)]
struct State {
    chains: Vec<ChainConfig>, // supported chains
    cron_state: Option<TaskScheduler>,
}

thread_local! {
    static CHAINS: RefCell<Vec<ChainConfig>> = RefCell::new(Vec::new());
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
}

#[update(name = "get_logs")]
#[candid_method(update, rename = "get_logs")]
async fn get() {
    get_logs().await
}


async fn get_logs() {
    ic_cdk::println!("getting logs now");
    // should traverse all chains, testing only ETH now
    // filter for events in our contract
    let filter = FilterBuilder::default()
        .address(vec![serde_json::from_str::<H160>(ETH_OMNIC_ADDR).unwrap()])
        .topics(
            Some(vec![serde_json::from_str::<H256>(EVENT_ENQUEUE_MSG).unwrap()]),
            None,
            None,
            None,
        )
        .from_block(BlockNumber::Number(7426080.into())) // omnic contract deploy block id
        .to_block(BlockNumber::Latest)
        .build();
    let w3 = match ICHttp::new(URL, None) {
        Ok(v) => { Web3::new(v) },
        Err(e) => { panic!() },
    };
    let logs = w3.eth().logs(filter).await.unwrap();
    for log in logs {
        ic_cdk::println!("{}", serde_json::to_string(&log).unwrap());
    }
}

#[heartbeat]
fn heartbeat() {
    for task in cron_ready_tasks() {
        let kind = task.get_payload::<Task>().expect("Serialization error");
        match kind {
            Task::GetLogs => {
                ic_cdk::spawn(get_logs());
            },
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