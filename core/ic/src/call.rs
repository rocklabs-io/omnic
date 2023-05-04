use candid::Principal;
use ic_cdk::api::call::{call, CallResult};

use crate::consts::{MAX_RESP_BYTES, CYCLES_PER_CALL};
use crate::types::Message;
use crate::chains::EVMChainClient;
use crate::traits::chain::HomeContract;

pub async fn call_to_canister(recipient: Principal, msg_hash: Vec<u8>, m: &Message) -> Result<String, String> {
    // call ic recipient canister
    let ret: CallResult<(Result<String, String>,)> = 
        call(recipient, "handle_message", (msg_hash, m.origin, m.sender.as_bytes(), m.nonce, m.body.clone(), )).await;
    match ret {
        Ok((res, )) => {
            match res {
                Ok(r) => {
                    ic_cdk::println!("handle_message success!");
                    return Ok(r);
                },
                Err(err) => {
                    ic_cdk::println!("handle_message failed: {:?}", err);
                    return Err(format!("handle_message failed: {:?}", err));
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

pub async fn call_to_chain(
    caller: String, 
    omnic_addr: String, 
    rpc: String, 
    dst_chain: u32, 
    msg_bytes: Vec<Vec<u8>>
) -> Result<String, String> {
    let client = EVMChainClient::new(rpc.clone(), omnic_addr.clone(), MAX_RESP_BYTES, CYCLES_PER_CALL)
        .map_err(|e| format!("init EVMChainClient failed: {:?}", e))?;
    client
        .dispatch_messages(caller, dst_chain, msg_bytes)
        .await
        .map(|txhash| {
            // ic_cdk::println!("dispatch_messages txhash: {:?}", hex::encode(txhash));
            // true
            hex::encode(txhash)
        })
        .map_err(|e| format!("dispatch_messages failed: {:?}", e))
}
