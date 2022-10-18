pkill dfx
sleep 3

dfx start --background --clean --enable-canister-http

dfx canister create proxy
dfx canister create demo

dfx build proxy
dfx build demo

dfx canister install proxy
dfx canister install demo

dfx canister --network ic call proxy add_chain "(5:nat32, vec {\"https://eth-goerli.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"; \"https://goerli.infura.io/v3/93ca33aa55d147f08666ac82d7cc69fd\"}, \"F98Ea5ca4BC350739FEd5D39582774D43ae403a8\", 7685333:nat64)"

dfx canister --network ic call proxy add_chain "(80001:nat32, vec{\"https://polygon-mumbai.g.alchemy.com/v2/0QCHDmgIEFRV48r1U1QbtOyFInib3ZAm\"}, \"4DCAA1f4f21333911394e83A075941474859E75E\", 28370114:nat64)"

dfx canister --network ic call proxy set_canister_addrs

dfx canister --network ic call proxy get_canister_addr "(variant {Evm})"

dfx canister --network ic call proxy get_chains

#echo "waiting for proxy to get latest root from eth..."
#sleep 30
#dfx canister call proxy get_latest_root "(5:nat32)"
