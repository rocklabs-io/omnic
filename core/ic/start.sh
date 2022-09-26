pkill dfx
sleep 3

dfx start --background --clean --enable-canister-http

dfx canister create proxy
dfx canister create demo

dfx build proxy
dfx build demo

dfx canister install proxy
dfx canister install demo

echo "waiting for proxy to get latest root from eth..."
sleep 30
dfx canister call proxy get_latest_root "(5:nat32)"
