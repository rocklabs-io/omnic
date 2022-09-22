use std::collections::BTreeMap;
use candid::types::{Serializer, Type};
use candid::{Int, Principal};
use ic_cdk::api::{call::CallResult, canister_balance};
use ic_cdk::export::candid::{candid_method, CandidType, Deserialize, Nat};
use ic_cdk_macros::{heartbeat, init, post_upgrade, pre_upgrade, query, update};
use ic_cron::task_scheduler::TaskScheduler;
use ic_cron::types::Iterations;
use ic_web3::ethabi::{decode, ParamType};
use ic_web3::types::U256;
use num_bigint::{BigInt, BigUint};
use omnic_bridge::pool::Pool;
use omnic_bridge::router::{Router, RouterInterfaces};
use omnic_bridge::token::Token as BrideToken;
use std::cell::RefCell;
use std::convert::TryInto;
use std::str::FromStr;

ic_cron::implement_cron!();

const OPERATION_ADD_LIQUIDITY: u8 = 1u8;
const OPERATION_REMOVE_LIQUIDITY: u8 = 2u8;
const OPERATION_SWAP: u8 = 3;

const OWNER: &'static str = "aaaaa-aa";

type Result<T> = std::result::Result<T, String>;

#[derive(CandidType, Deserialize, Default)]
pub struct BridgeAddress<T> {
    bridges: BTreeMap<u32, T>
}

impl<T: std::clone::Clone > BridgeAddress<T> {
    pub fn new() -> Self {
        BridgeAddress {
            bridges: BTreeMap::new(),
        }
    }
    pub fn get_bridge_addr(&self, src_chain: u32) -> Option<T> {
        self.bridges.get(&src_chain).cloned()
    }

    pub fn is_bridge_exist(&self, src_chain: u32) -> bool {
        self.bridges.contains_key(&src_chain)
    }

    pub fn add_bridge_addr(&mut self, src_chain: u32, bridge_addr: T) {
        self.bridges.entry(src_chain).or_insert(bridge_addr);
    }
 
    pub fn remove_bridge_addr(&mut self, src_chain: u32) -> T {
        self.bridges.remove(&src_chain).unwrap()
    }
}

thread_local! {
    static ROUTER: RefCell<Router> = RefCell::new(Router::new());
    static BRIDGES: RefCell<BridgeAddress<Vec<u8>>> = RefCell::new(BridgeAddress::new());
}

#[update(name = "process_message")]
#[candid_method(update, rename = "processMessage")]
fn process_message(src_chain: u32, sender: Vec<u8>, nonce: u32, payload: Vec<u8>) -> Result<bool> {
    let t = vec![ParamType::Uint(8)];
    let d = decode(&t, &payload).map_err(|e| format!("payload decode error"))?;
    let operation_type: u8 = d[0]
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
            let mut buffer1 = [0u8; 32];
            let mut buffer2 = [0u8; 32];
            src_pool_id.to_little_endian(&mut buffer1);
            amount.to_little_endian(&mut buffer2);
            r.add_liquidity(
                Nat::from(src_chain),
                // Nat::from(BigUint::from_slice(src_pool_id.as_ref())),
                Nat::from(BigUint::from_bytes_le(&buffer1)),
                sender,
                Nat::from(BigUint::from_bytes_le(&buffer2)),
            )
            .map_err(|_| format!("add liquidity failed"))
        })
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
            let mut buffer1 = [0u8; 32];
            let mut buffer2 = [0u8; 32];
            src_pool_id.to_little_endian(&mut buffer1);
            amount.to_little_endian(&mut buffer2);
            r.remove_liquidity(
                Nat::from(src_chain),
                Nat::from(BigUint::from_bytes_le(&buffer1)),
                sender,
                Nat::from(BigUint::from_bytes_le(&buffer2)),
            )
            .map_err(|_| format!("remove liquidity failed"))
        })
    } else if operation_type == OPERATION_SWAP {
        //TODO
        Err("unsupported!".to_string())
    } else {
        Err("unsupported!".to_string())
    }
}

#[update(name = "send_message")]
#[candid_method(update, rename = "sendMessage")]
fn send_message() -> Result<bool> {
    //TODO
    Ok(false)
}

#[update(name = "create_pool")]
#[candid_method(update, rename = "createPool")]
fn create_pool(src_chain: Nat, src_pool_id: Nat, shared_decimals: u8) -> Result<bool> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    ROUTER.with(|router| {
        let mut r = router.borrow_mut();
        let pool_id: Nat = r.get_pools_length();
        let pool: Pool = Pool::new(pool_id.clone(), shared_decimals);
        r.pools.entry(pool_id.clone()).or_insert(pool);
        r.pool_ids
            .entry(src_chain)
            .or_default()
            .insert(src_pool_id, pool_id.clone());
        Ok(true)
    })
}

#[update(name = "add_supported_token")]
#[candid_method(update, rename = "addSupportedToken")]
fn add_supported_token(src_chain: Nat, src_pool_id: Nat, name: String, symbol: String, decimals: u8) -> Result<bool> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    ROUTER.with(|router| {
        let mut r = router.borrow_mut();
        let pool_id: Nat = r.get_pool_id(src_chain.clone(), src_pool_id.clone()).map_err(|e| format!("{}", e))?;
        let mut pool: Pool = r.get_pool(pool_id.clone()).map_err(|e| format!("{}", e))?;
        let token_num:Nat = pool.get_token_count();
        let token: BrideToken = BrideToken::new(src_chain, src_pool_id, name, symbol, decimals);
        pool.token_info.entry(token_num).or_insert(token);
        r.pools.entry(pool_id.clone()).or_insert(pool); //update pool
        Ok(true)
    })
}

#[update(name = "add_bridge_addr")]
#[candid_method(update, rename = "addBridgeAddr")]
fn add_bridge_addr(src_chain: u32, birdge_addr: Vec<u8>) -> Result<bool> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    BRIDGES.with(|bridge_addrs| {
        let mut b = bridge_addrs.borrow_mut();
        b.add_bridge_addr(src_chain, birdge_addr);
        Ok(true)
    })
}

#[update(name = "remove_bridge_addr")]
#[candid_method(update, rename = "removeBridgeAddr")]
fn remove_bridge_addr(src_chain: u32) -> Result<Vec<u8>> {
    let caller: Principal = ic_cdk::caller();
    let owner: Principal = Principal::from_text(OWNER).unwrap();
    assert_eq!(caller, owner);

    BRIDGES.with(|bridge_addrs| {
        let mut b = bridge_addrs.borrow_mut();
        Ok(b.remove_bridge_addr(src_chain))
    })
}

#[update(name = "get_bridge_addr")]
#[candid_method(update, rename = "getBridgeAddr")]
fn get_bridge_addr(src_chain: u32) -> Result<Vec<u8>> {

    BRIDGES.with(|bridge_addrs| {
        let b = bridge_addrs.borrow();
        b.get_bridge_addr(src_chain).ok_or(format!("not bridge address in {} chain", src_chain))
    })
}

#[update(name = "is_bridge_addr_exist")]
#[candid_method(update, rename = "isBridgeAddrExist")]
fn is_bridge_addr_exist(src_chain: u32) -> Result<bool> {
    BRIDGES.with(|bridge_addrs| {
        let b = bridge_addrs.borrow();
        Ok(b.is_bridge_exist(src_chain))
    })
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
