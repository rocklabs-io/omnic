use crate::error::{Error, Result};
use crate::pool::{Error as PoolError, Pool};
use crate::token::{Error as TokenError, Operation, Token};
use candid::{types::number::Nat, CandidType, Deserialize};
use std::collections::BTreeMap;

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Router {
    pub src_chain: u32,
    pub bridge_addr: String, // bridge address on src chain
    pub pools: BTreeMap<u32, Pool>, // src_pool_id -> Pool
    pub token_pool: BTreeMap<String, u32>, // token_address -> pool_id
}

impl Router {
    pub fn new(
        src_chain: u32,
        bridge_addr: String,
    ) -> Self {
        Router {
            src_chain,
            bridge_addr,
            pools: BTreeMap::new(),
            token_pool: BTreeMap::new(),
        }
    }

    pub fn pool_exists(&self, token_addr: &str) -> bool {
        match self.token_pool.get(token_addr) {
            Some(_) => {
                true
            },
            None => {
                false
            },
        }
    }

    pub fn pool_by_token_address(&self, token_addr: String) -> Pool {
        let pool_id = match self.token_pool.get(&token_addr) {
            Some(id) => {
                id.clone()
            },
            None => {
                unreachable!();
            },
        };
        self.get_pool(pool_id)
    }

    pub fn create_pool(&mut self, 
        pool_id: u32,
        pool_address: String,
        shared_decimals: u8,
        local_decimals: u8,
        token: Token
    ) {
        if self.pool_exists(&token.address) {
            return;
        }
        let pool = Pool::new(
            self.src_chain,
            pool_id,
            pool_address,
            shared_decimals,
            local_decimals,
            token
        );
        self.pools.entry(pool_id).or_insert(pool);
    }

    pub fn get_pool(&self, pool_id: u32) -> Pool {
        match self.pools.get(&pool_id) {
            Some(p) => p.clone(),
            None => unreachable!(),
        }
    }

    pub fn get_pool_token(&self, pool_id: u32) -> Token {
        let pool = match self.pools.get(&pool_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        pool.token.clone()
    }

    pub fn add_liquidity(&mut self, pool_id: u32, amount_ld: u128) {
        let mut pool = match self.pools.get_mut(&pool_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        pool.add_liquidity(amount_ld);
    }

    pub fn remove_liquidity(&mut self, pool_id: u32, amount_ld: u128) {
        let mut pool = match self.pools.get_mut(&pool_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        if pool.enough_liquidity(amount_ld) {
            pool.remove_liquidity(amount_ld)
        }
    }

    pub fn enough_liquidity(&self, pool_id: u32, amount_ld: u128) -> bool {
        let pool = match self.pools.get(&pool_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        pool.enough_liquidity(amount_ld)
    }

    pub fn amount_ld(&self, pool_id: u32, amount_sd: u128) -> u128 {
        let pool = match self.pools.get(&pool_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        pool.amount_ld(amount_sd)
    }

    pub fn amount_sd(&self, pool_id: u32, amount_ld: u128) -> u128 {
        let pool = match self.pools.get(&pool_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        pool.amount_sd(amount_ld)
    }
}

// chain_id -> Router
#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct BridgeRouters(BTreeMap<u32, Router>); 

impl BridgeRouters {
    pub fn new() -> Self {
        BridgeRouters(BTreeMap::new())
    }

    pub fn pool_exists(&self, chain_id: u32, token_addr: String) -> bool {
        let router = match self.0.get(&chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        router.pool_exists(&token_addr)
    }

    pub fn pool_by_token_address(&self, chain_id: u32, token_addr: String) -> Pool {
        let router = match self.0.get(&chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        router.pool_by_token_address(token_addr)
    }

    pub fn get_pool(&self, chain_id: u32, pool_id: u32) -> Pool {
        let router = match self.0.get(&chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        router.get_pool(pool_id)
    }

    pub fn get_pool_token(&self, chain_id: u32, pool_id: u32) -> Token {
        let router = match self.0.get(&chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        router.get_pool_token(pool_id)
    }

    pub fn amount_ld(&self, chain_id: u32, pool_id: u32, amount_sd: u128) -> u128 {
        let router = match self.0.get(&chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        router.amount_ld(pool_id, amount_sd)
    }

    pub fn create_pool(
        &mut self, 
        src_chain: u32, 
        pool_id: u32, 
        pool_address: String,
        shared_decimals: u8,
        local_decimals: u8,
        token: Token
    ) {
        let mut router = match self.0.get_mut(&src_chain) {
            Some(p) => p,
            None => unreachable!(),
        };
        router.create_pool(pool_id, pool_address, shared_decimals, local_decimals, token);
    }

    pub fn add_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        _to: String,
        amount_ld: u128,
    ) {
        let mut router = match self.0.get_mut(&src_chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        router.add_liquidity(src_pool_id, amount_ld);
    }

    pub fn remove_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        _from: String,
        amount_ld: u128,
    ) {
        let mut router = match self.0.get_mut(&src_chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        router.remove_liquidity(src_pool_id, amount_ld);
    }

    pub fn swap(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        dst_chain_id: u32,
        dst_pool_id: u32,
        amount_sd: u128,
    ) {
        let mut binding = self.0.clone();
        let mut src_router = match binding.get_mut(&src_chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        let mut dst_router = match self.0.get_mut(&dst_chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        let dst_amount_ld = dst_router.amount_ld(dst_pool_id, amount_sd);
        if dst_router.enough_liquidity(dst_pool_id, dst_amount_ld) {
            let src_amount_ld = src_router.amount_ld(src_pool_id, amount_sd);
            src_router.add_liquidity(src_pool_id, src_amount_ld);
            dst_router.remove_liquidity(dst_pool_id, dst_amount_ld);
        }
    }

    pub fn check_swap(
        &mut self,
        src_chain_id: u32,
        src_pool_id: u32,
        dst_chain_id: u32,
        dst_pool_id: u32,
        amount_sd: u128,
    ) -> bool {
        let src_router = match self.0.get(&src_chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        let dst_router = match self.0.get(&src_chain_id) {
            Some(p) => p,
            None => unreachable!(),
        };
        let dst_amount_ld = dst_router.amount_ld(dst_pool_id, amount_sd);
        dst_router.enough_liquidity(dst_pool_id, dst_amount_ld)
    }
}
