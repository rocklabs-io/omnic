pkill dfx
sleep 3

dfx start --background --clean --enable-canister-http

dfx canister create proxy
dfx canister create demo

dfx build proxy
dfx build demo

dfx canister install proxy
dfx canister install demo

dfx canister --network ic call proxy add_chain "(5:nat32, vec {\"https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"}, principal \"$(dfx canister --network ic id goerli-gateway)\", \"c7D718dC3C9248c91813A98dCbFEC6CF57619520\", 7685333:nat64)"

dfx canister --network ic call proxy add_chain "(80001:nat32, vec {\"https://polygon-mumbai.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"}, principal \"$(dfx canister --network ic id mumbai-gateway)\", \"2F711bEbA7a30242f4ba24544eA3869815c41413\", 28370114:nat64)"

dfx canister --network ic call proxy set_canister_addrs

dfx canister --network ic call proxy get_canister_addr "(variant {Evm})"

dfx canister --network ic call proxy get_chains

#echo "waiting for proxy to get latest root from eth..."
#sleep 30
#dfx canister call proxy get_latest_root "(5:nat32)"
