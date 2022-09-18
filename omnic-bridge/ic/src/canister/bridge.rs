use candid::types::{Serializer, Type};
use candid::{Int, Principal};
use ic_cdk::api::{call::CallResult, canister_balance};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Nat};
use ic_cdk_macros::{heartbeat, init, post_upgrade, pre_upgrade, query, update};
use ic_cron::task_scheduler::TaskScheduler;
use ic_cron::types::Iterations;
use ic_web3::ethabi::{decode, ParamType, Token};
use ic_web3::types::U256;
use num_bigint::{BigInt, BigUint};
use omnic_bridge::router::Router;
use std::cell::RefCell;
use std::convert::TryInto;
use omnic_bridge::router::RouterInterfaces;

ic_cron::implement_cron!();

const OPERATION_ADD_LIQUIDITY: usize = 1;
const OPERATION_REMOVE_LIQUIDITY: usize = 2;
const OPERATION_SWAP: usize = 3;

thread_local! {
    static ROUTER: RefCell<Router> = RefCell::new(Router::new());
}

#[update(name = "process_message")]
#[candid_method(update, rename = "processMessage")]
fn process_message(
    src_chain: u32,
    sender: Vec<u8>,
    nonce: u32,
    payload: Vec<u8>,
) -> Result<bool, String> {
    let t = vec![ParamType::Uint(8)];
    let d = decode(&t, &payload).map_err(|e| format!("payload decode error"))?;
    let operation_type = d[0]
        .clone()
        .into_uint()
        .ok_or("can not convert src_chain to U256")?
        .try_into()
        .map_err(|_| format!("convert U256 to u8 failed"))?;
    if operation_type == OPERATION_ADD_LIQUIDITY {
        let types = vec![
            ParamType::Uint(8),
            ParamType::Uint(16),
            ParamType::Uint(256),
            ParamType::Uint(256),
        ];
        let d = decode(&types, &payload).map_err(|e| format!("payload decode error"))?;
        let src_pool_id: U256 = d[2]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?;
        let amount: U256 = d[3]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?;

        ROUTER.with(|router| {
            let mut r = router.borrow_mut();
            let mut buffer1 = [0u8 ..512];
            let mut buffer2 = [];
            r.add_liquidity(
                Nat::from(src_chain),
                Nat::from(BigUint::from_bytes_le(src_pool_id.to_little_endian(&mut buffer1.as_mut_slice()))),
                sender,
                Nat::from(BigUint::from_bytes_le(amount.to_little_endian(&mut buffer2))),
            )
        });
    } else if operation_type == OPERATION_REMOVE_LIQUIDITY {
        let types = vec![
            ParamType::Uint(8),
            ParamType::Uint(16),
            ParamType::Uint(256),
            ParamType::Uint(256),
        ];
        let d = decode(&types, &payload).map_err(|e| format!("payload decode error"))?;
        let src_pool_id: U256 = d[2]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?;
        let amount: U256 = d[3]
            .clone()
            .into_uint()
            .ok_or("can not convert src_chain to U256".to_string())?;

        ROUTER.with(|router| {
            let mut r = router.borrow_mut();
            let mut buffer1 = [];
            let mut buffer2 = [];
            r.remove_liquidity(
                Nat::from(src_chain),
                Nat::from(BigUint::from_bytes_le(src_pool_id.to_little_endian(&mut buffer1))),
                sender,
                Nat::from(BigUint::from_bytes_le(amount.to_little_endian(&mut buffer2))),
            )
        });
    } else if operation_type == OPERATION_SWAP {
        //TODO
    }
    Ok(true)
}

#[update(name = "send_message")]
#[candid_method(update, rename = "sendMessage")]
fn send_message() -> bool {
    //TODO
    false
}

#[update(name = "create_pool")]
#[candid_method(update, rename = "createPool")]
fn create_pool() -> bool {
    //TODO
    false
}

#[update(name = "add_supported_token")]
#[candid_method(update, rename = "addSupportedToken")]
fn add_supported_token() -> bool {
    //TODO
    false
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
