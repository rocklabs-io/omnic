use candid::Principal;
use ic_cdk::api::call::{call, CallResult};

use crate::consts::{MAX_RESP_BYTES, CYCLES_PER_CALL};
use crate::types::Message;
use crate::chains::EVMChainClient;
use crate::traits::chain::HomeContract;

pub async fn call_to_canister(recipient: Principal, m: &Message) -> Result<bool, String> {
    // call ic recipient canister
    let ret: CallResult<(Result<bool, String>,)> = 
        call(recipient, "handle_message", (m.origin, m.sender.as_bytes(), m.nonce, m.body.clone(), )).await;
    match ret {
        Ok((res, )) => {
            match res {
                Ok(_) => {
                    ic_cdk::println!("handle_message success!");
                },
                Err(err) => {
                    ic_cdk::println!("handle_message failed: {:?}", err);
                    return Err(format!("handle_message failed: {:?}", err));
                }
            }
            // message delivered
            Ok(true)
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
    msg_bytes: Vec<u8>
) -> Result<bool, String> {
    let client = EVMChainClient::new(rpc.clone(), omnic_addr.clone(), MAX_RESP_BYTES, CYCLES_PER_CALL)
        .map_err(|e| format!("init EVMChainClient failed: {:?}", e))?;
    client
        .dispatch_message(caller, dst_chain, msg_bytes)
        .await
        .map(|txhash| {
            ic_cdk::println!("dispatch_message txhash: {:?}", hex::encode(txhash));
            true
        })
        .map_err(|e| format!("dispatch_message failed: {:?}", e))
}
