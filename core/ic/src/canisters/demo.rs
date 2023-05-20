use std::str;
use ic_cdk_macros::{query, update};
use ic_cdk::export::candid::{candid_method};
use ic_cdk::export::Principal;


#[update(name = "handle_message")]
#[candid_method(update, rename = "handle_message")]
fn handle_message(msg_type: u8, msg_hash: Vec<u8>, origin: u32, sender: Vec<u8>, nonce: u32, body: Vec<u8>) -> Result<bool, String> {
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