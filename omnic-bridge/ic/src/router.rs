use crate::token::Token;
use crate::pool::Pool;
use candid::{CandidType, Deserialize};
use std::collections::BTreeMap;
use std::cell::RefCell;

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Router {
    src_chain: u32,
    bridge_addr: String, // bridge address on src chain
    pools: BTreeMap<u32, Pool>, // src_pool_id -> Pool
    token_pool: BTreeMap<String, u32>, // token_address -> pool_id
}

// get some info from router
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

    pub fn remove_pool(&mut self, pool_id: u32) {
        let pool = self.pool_by_id(pool_id);
        self.pools.remove(&pool_id);
        self.token_pool.remove(&pool.token_address());
    }

    pub fn pool_count(&self) -> u32 {
        self.pools.len() as u32
    }

    pub fn src_chain(&self) -> u32 {
        self.src_chain
    }

    pub fn bridge_addr(&self) -> String {
        self.bridge_addr.clone()
    }

    pub fn pool_exists(&self, token_addr: &str) -> bool {
        self.token_pool.get(token_addr).is_some()
    }

    pub fn pool_by_token_address(&self, token_addr: &str) -> Pool {
        let pool_id = self.token_pool.get(token_addr).cloned().expect("no pool! Please check the token address input.");
        self.pool_by_id(pool_id)
    }

    pub fn pool_by_id(&self, pool_id: u32) -> Pool {
        self.pools.get(&pool_id).cloned().expect("no pool! Please check the pool_id.")
    }

    pub fn pool_token(&self, pool_id: u32) -> Token {
        let pool = self.pools.get(&pool_id).cloned().expect("no pool! Please check the input pool_id");
        pool.token()
    }
}

// set function for Router
impl Router {
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
        let token_addr = token.address.clone();
        let pool = Pool::new(
            self.src_chain,
            pool_id,
            pool_address,
            shared_decimals,
            local_decimals,
            token
        );
        self.pools.entry(pool_id).or_insert(pool);
        self.token_pool.entry(token_addr).or_insert(pool_id);
    }

    pub fn add_liquidity(&mut self, pool_id: u32, amount_ld: u128) {
        let pool = self.pools.get_mut(&pool_id).expect("no pool! Please check the input pool_id");
        pool.add_liquidity(amount_ld)
    }

    pub fn remove_liquidity(&mut self, pool_id: u32, amount_ld: u128) -> bool {
        let pool = self.pools.get_mut(&pool_id).expect("no pool! Please check the input pool_id");
        if pool.enough_liquidity(amount_ld) {
            pool.remove_liquidity(amount_ld);
            true
        } else {
            false
        }
    }

    pub fn enough_liquidity(&self, pool_id: u32, amount_ld: u128) -> bool {
        let pool = self.pools.get(&pool_id).expect("no pool! Please check the input pool_id");
        pool.enough_liquidity(amount_ld)
    }

    pub fn amount_ld(&self, pool_id: u32, amount_sd: u128) -> u128 {
        let pool = self.pools.get(&pool_id).expect("no pool! Please check the input pool_id");
        pool.amount_ld(amount_sd)
    }

    pub fn amount_sd(&self, pool_id: u32, amount_ld: u128) -> u128 {
        let pool = self.pools.get(&pool_id).expect("no pool! Please check the input pool_id");
        pool.amount_sd(amount_ld)
    }
}

// chain_id -> Router
#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct BridgeRouters(BTreeMap<u32, RefCell<Router>>); 

impl BridgeRouters {
    pub fn new() -> Self {
        BridgeRouters(BTreeMap::new())
    }

    pub fn chain_exists(&self, chain_id: u32) -> bool {
        self.0.contains_key(&chain_id)
    }

    pub fn get_router(&self, chain_id: u32) -> Router {
        let router = self.0.get(&chain_id).expect("router not found");
        router.borrow().clone()
    }

    pub fn bridge_addr(&self, chain_id: u32) -> String {
        let router = self.0.get(&chain_id).expect("router not found");
        router.borrow().bridge_addr()
    }

    pub fn remove_pool(&mut self, chain_id: u32, pool_id: u32) {
        let router = self.0.get(&chain_id).expect("router not found");
        router.borrow_mut().remove_pool(pool_id);
    }

    pub fn pool_count(&self, chain_id: u32) -> u32 {
        let router = self.0.get(&chain_id).expect("no router on this chain!");
        router.borrow().pool_count()
    }

    pub fn pool_exists(&self, chain_id: u32, token_addr: &str) -> bool {
        let router = self.0.get(&chain_id).expect("no router on this chain!");
        router.borrow().pool_exists(token_addr)
    }

    pub fn pool_by_token_address(&self, chain_id: u32, token_addr: &str) -> Pool {
        let router = self.0.get(&chain_id).expect("no router on this chain!");
        router.borrow().pool_by_token_address(token_addr)
    }

    pub fn pool_by_id(&self, chain_id: u32, pool_id: u32) -> Pool {
        let router = self.0.get(&chain_id).expect("no router on this chain!");
        router.borrow().pool_by_id(pool_id)
    }

    pub fn pool_token(&self, chain_id: u32, pool_id: u32) -> Token {
        let router = self.0.get(&chain_id).expect("no router on this chain!");
        router.borrow().pool_token(pool_id)
    }

    pub fn amount_ld(&self, chain_id: u32, pool_id: u32, amount_sd: u128) -> u128 {
        let router = self.0.get(&chain_id).expect("no router on this chain!");
        router.borrow().amount_ld(pool_id, amount_sd)
    }

    pub fn add_chain(&mut self, chain_id: u32, bridge_addr: String) {
        let r = Router::new(chain_id, bridge_addr);
        self.0.entry(chain_id).or_insert(RefCell::new(r));
    }

    pub fn create_pool(
        &self, 
        chain_id: u32, 
        pool_id: u32, 
        pool_address: String,
        shared_decimals: u8,
        local_decimals: u8,
        token: Token
    ) {
        let router = self.0.get(&chain_id).expect("BridgeRouter: no router on this chain!");
        router.borrow_mut().create_pool(pool_id, pool_address, shared_decimals, local_decimals, token);
    }

    pub fn add_liquidity(
        &self,
        src_chain_id: u32,
        src_pool_id: u32,
        amount_ld: u128,
    ) {
        let router = self.0.get(&src_chain_id).expect("BridgeRouter: no router on this chain!");
        router.borrow_mut().add_liquidity(src_pool_id, amount_ld);
    }

    pub fn remove_liquidity(
        &self,
        src_chain_id: u32,
        src_pool_id: u32,
        amount_ld: u128,
    ) {
        let router = self.0.get(&src_chain_id).expect("BridgeRouter: no router on this chain!");
        router.borrow_mut().remove_liquidity(src_pool_id, amount_ld);
    }

    pub fn swap(
        &self,
        src_chain_id: u32,
        src_pool_id: u32,
        dst_chain_id: u32,
        dst_pool_id: u32,
        amount_sd: u128,
    ) {
        let src_router = self.0.get(&src_chain_id).expect("BridgeRouter: no router on this chain!");
        let dst_router = self.0.get(&dst_chain_id).expect("BridgeRouter: no router on this chain!");
        let dst_amount_ld = dst_router.borrow().amount_ld(dst_pool_id, amount_sd);
        dst_router.borrow().enough_liquidity(dst_pool_id, dst_amount_ld)
            .then(move || {
                let src_amount_ld = src_router.borrow().amount_ld(src_pool_id, amount_sd);
                src_router.borrow_mut().add_liquidity(src_pool_id, src_amount_ld);
                // TODO: how to handle transaction failure with remote swap ?
                dst_router.borrow_mut().remove_liquidity(dst_pool_id, dst_amount_ld);
            });
    }

    pub fn check_swap(
        &self,
        dst_chain_id: u32,
        dst_pool_id: u32,
        amount_sd: u128,
    ) -> bool {
        let dst_router = self.0.get(&dst_chain_id).expect("BridgeRouter: no router on this chain!");
        let dst_amount_ld = dst_router.borrow().amount_ld(dst_pool_id, amount_sd);
        dst_router.borrow_mut().enough_liquidity(dst_pool_id, dst_amount_ld)
    }
}
