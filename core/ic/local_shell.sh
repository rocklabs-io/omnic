OMNIC=""
START_BLOCK=

dfx create --all
dfx build --all

# reinstall demo if any
dfx canister install demo --mode=reinstall

#reinstall proxy if any
dfx canister install proxy --mode=reinstall
dfx canister call proxy add_chain "(80001:nat32, vec {\"https://polygon-mumbai.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"}, principal \"$(dfx canister id mumbai-gateway)\", \"$OMNIC\", $START_BLOCK:nat64, 10_000_000_000: nat64, 10:nat64)"
dfx canister call proxy set_canister_addrs
dfx canister call proxy add_owner "(principal \"$(dfx canister id mumbai-gateway)\")"

dfx canister call proxy get_canister_addr "(variant {Evm})"
dfx canister call proxy get_chains


#reintall mumbai-gateway
dfx canister install mumbai-gateway --mode=reinstall
dfx canister call mumbai-gateway add_chain "(80001:nat32, vec {\"https://polygon-mumbai.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"}, \"$OMNIC\", $START_BLOCK:nat64)"

