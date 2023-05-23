use std::str;
use ic_cdk_macros::{query, update};
use ic_cdk::export::candid::{candid_method};
use ic_cdk::api::call::CallResult;
use ic_cdk::export::Principal;

const MESSAGE_TYPE_SYN: u8 = 0;
const MESSAGE_TYPE_ACK: u8 = 1;
const MESSAGE_TYPE_FAIL_ACK: u8 = 2;

const OMNIC_PROXY_CANISTER: &str = "rkp4c-7iaaa-aaaaa-aaaca-cai";
const PLOYGON_DEMO_CONTRACT: &str = "2B4B4618B29c6E994A20A1Eaa926710a920d5176";

#[update(name = "handle_message")]
#[candid_method(update, rename = "handle_message")]
async fn handle_message(msg_type: u8, msg_hash: Vec<u8>, origin: u32, sender: Vec<u8>, nonce: u64, body: Vec<u8>) -> Result<String, String> {
    ic_cdk::println!("demo app got message: {:?}", (origin, nonce, hex::encode(&sender), str::from_utf8(&body)));
    let mut recipient: [u8;32] = [0;32];
    if sender.len() == recipient.len() {
        recipient.copy_from_slice(&sender);
    }
    let ret = _send_receipt(MESSAGE_TYPE_ACK, origin, recipient, "success!".to_string().as_bytes().to_vec()).await;
    match ret {
        Ok(_) => Ok("send receipt success".into()),
        Err(e) => Err(format!("send receipt error, {}", e))
    }
}

// send receipt to evm through proxy interface: SendMessage
async fn _send_receipt(msg_type: u8, dst_chain: u32, recipient: [u8;32], payload: Vec<u8>) -> Result<bool, String > {
    let proxy_canister = Principal::from_text(OMNIC_PROXY_CANISTER).map_err(|_| format!("proxy canister wrong."))?;
    let transfer_res: CallResult<(std::result::Result<bool, String>,)> =
        ic_cdk::call(
            proxy_canister,
            "send_message",
            (msg_type, dst_chain, recipient, payload),
        )
        .await;
    // let ret = transfer_res.map_or_else(|_| false, |r| r.0.map_or(false, |v| v));
    match transfer_res {
        Ok((res, )) => {
            match res {
                Ok(r) => {
                    ic_cdk::println!("send message success!");
                    return Ok(r);
                },
                Err(err) => {
                    ic_cdk::println!("send message failed: {:?}", err);
                    return Err(format!("send message failed internal: {:?}", err));
                }
            }
            // message delivered
            // Ok(true)
        },
        Err((_code, msg)) => {
            ic_cdk::println!("call app canister failed: {:?}", (_code, msg.clone()));
            // message delivery failed
            Err(format!("call app canister failed: {:?}", (_code, msg)))
        }
    }
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