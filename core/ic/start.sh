pkill dfx
sleep 3

dfx start --background --clean --enable-canister-http

dfx canister create proxy
dfx canister create demo

dfx build proxy
dfx build demo

dfx canister install proxy
dfx canister install demo

dfx canister call proxy add_chain "(5:nat32, vec {\"https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"; \"https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd\"}, \"0312504E22B40A6f03FcCFEA0C8c0e9Ad3E36918\", 7558863:nat64)"

dfx canister call proxy add_chain "(80001:nat32, vec{\"https://polygon-mumbai.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"}, \"430365a65eAC8626AFf329e5B3a549f6C13D0c26\", 28161966:nat64)"

dfx canister call proxy set_canister_addrs

dfx canister call proxy get_canister_addr "(variant {Evm})"

dfx canister call proxy get_chains

#echo "waiting for proxy to get latest root from eth..."
#sleep 30
#dfx canister call proxy get_latest_root "(5:nat32)"
