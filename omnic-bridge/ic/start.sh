
dfx build --netowrk ic bridge

dfx canister --network ic install bridge

dfx canister --network ic call bridge set_canister_addr

dfx canister --network ic call bridge set_omnic "(principal \"$(dfx canister --network ic id proxy)\")"

dfx canister --network ic call bridge add_chain "(5:nat32, \"12F12F6917804c72FED66E118f99a78074F1BdE4\")"

dfx canister --network ic call bridge add_chain "(0:nat32, \"\")"

dfx canister --network ic call bridge create_pool "(\"$(dfx canister --network ic id usdt)\", 6:nat8)"
