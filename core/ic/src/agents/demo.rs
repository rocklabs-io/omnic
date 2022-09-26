/*
omnic proxy canister:
    fetch_root: fetch merkel roots from all supported chains and insert to chain state
*/

use std::cell::{RefCell};
use std::collections::HashMap;
use std::str::FromStr;
use std::str;

use ic_web3::Web3;
use ic_web3::contract::{Contract, Options};
use ic_web3::types::{H256, Address, BlockNumber, BlockId};
use ic_web3::transports::ICHttp;
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update, heartbeat};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize};
use ic_cdk::api::call::{call, CallResult};
use ic_cdk::export::Principal;


#[update(name = "handle_message")]
#[candid_method(update, rename = "handle_message")]
fn handle_message(origin: u32, nonce: u32, sender: Vec<u8>, body: Vec<u8>) -> Result<bool, String> {
    ic_cdk::println!("demo app got message: {:?}", (origin, nonce, hex::encode(&sender), str::from_utf8(&body)));
    Ok(true)
}

#[query(name = "hex_pid")]
#[candid_method(query, rename = "hex_pid")]
fn hex_pid(pid: Principal) {
    ic_cdk::println!("pid len: {}", pid.clone().as_slice().len());
    ic_cdk::println!("hex: {:?}", hex::encode(pid.as_slice()));
}


fn main() {
    candid::export_service!();
    std::print!("{}", __export_service());
}