use ic_cdk::api::{call::CallResult, canister_balance};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Int, Nat};
use ic_cdk::export::Principal;


#[update(name = "handle_message")]
#[candid_method(update, rename = "handle_message")]
async fn handle_message(payload: Vec<u8>) -> String {
    ic_cdk::println!("demo app canister received message: {:?}", &payload);
    format!("msg: {:?}", payload)
}

fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}