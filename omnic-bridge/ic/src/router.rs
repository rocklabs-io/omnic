use crate::error::{Error, Result};
use crate::pool::{Error as PoolError, Pool};
use crate::token::{Error as TokenError, Operation};
use ic_cdk::export::candid::{CandidType, Deserialize, Nat};
use std::collections::BTreeMap;


pub trait RouterInterfaces {
    type AccountItem;
    fn add_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        to: Self::AccountItem,
        amount: u128,
    ) -> Result<bool>;
    fn remove_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        from: Self::AccountItem,
        amount: u128,
    ) -> Result<bool>;
    fn swap(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        dst_chain_id: u32,
        dst_pool_id: u32,
        amount: u128,
    ) -> Result<bool>;
    // check if the destination pool has enough liquidity for this swap
    fn check_swap(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        dst_chain_id: u32,
        dst_pool_id: u32,
        amount: u128,
    ) -> Result<bool>;
}

// chain_id -> Router
#[derive(Deserialize, CandidType, Clone, Debug)]
pub type BridgeRouters = BTreeMap<u32, Router>; 

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Router {
    pub src_chain: u32;
    pub bridge_addr: String; // bridge address on src chain
    pub pools: BTreeMap<u32, Pool>; // src_pool_id -> Pool
}


impl Router {
    pub fn new() -> Self {
        Router {
            
        }
    }

    
}

impl RouterInterfaces for BridgeRouters {
    fn add_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        to: String,
        amount: u128,
    ) -> Result<bool> {
        
    }

    fn remove_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        from: String,
        amount: u32,
    ) -> Result<bool> {
        
    }

    fn swap(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        dst_chain_id: u32,
        dst_pool_id: u32,
        amount: u128,
    ) -> Result<bool> {
        
    }

    fn check_swap(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        dst_chain_id: u32,
        dst_pool_id: u32,
        amount: u128,
    ) -> Result<bool> {

    }
}
