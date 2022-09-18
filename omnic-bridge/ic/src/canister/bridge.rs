use std::cell::RefCell;
use candid::{Int, Principal};
use candid::types::{Serializer, Type};
use ic_cdk::api::{call::CallResult, canister_balance};
use ic_cdk::export::candid::{candid_method, Nat, CandidType, Deserialize};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cron::task_scheduler::TaskScheduler;
use ic_cron::types::Iterations;
use omnic_bridge::router::Router;
use omnic_bridge::token::{Operation, Token};


ic_cron::implement_cron!();

const OPERATION_ADD_LIQUIDITY: usize = 1;
const OPERATION_REMOVE_LIQUIDITY: usize = 2;
const OPERATION_SWAP: usize = 3;

thread_local! {
    static TOUTER: RefCell<Router> = RefCell::new(Router::new());
}

#[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    // The line below generates did types and service definition from the
    // methods annotated with `candid_method` above. The definition is then
    // obtained with `__export_service()`.
    candid::export_service!();
    std::print!("{}", __export_service());
}

#[cfg(any(target_arch = "wasm32", test))]
fn main() {}