pkill dfx
sleep 3

dfx start --background --clean --enable-canister-http

dfx canister create proxy
dfx canister create demo

dfx build proxy
dfx build demo

dfx canister install proxy
dfx canister install demo

dfx canister call proxy add_chain "(5:nat32, vec {\"https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"; \"https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd\"}, \"92Dc043616E1FC9f7817b4a448e496487E369634\", 7685333:nat64)"

dfx canister call proxy add_chain "(80001:nat32, vec{\"https://polygon-mumbai.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"}, \"9FC058A2B96F5ff4A63e5BB0855f413e833c1fA6\", 28370114:nat64)"

dfx canister call proxy set_canister_addrs

dfx canister call proxy get_canister_addr "(variant {Evm})"

dfx canister call proxy get_chains

#echo "waiting for proxy to get latest root from eth..."
#sleep 30
#dfx canister call proxy get_latest_root "(5:nat32)"
