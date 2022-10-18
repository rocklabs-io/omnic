
dfx build --netowrk ic bridge

dfx canister --network ic install bridge

dfx canister --network ic call bridge set_canister_addr

dfx canister --network ic call bridge set_omnic "(principal \"$(dfx canister --network ic id proxy)\")"

dfx canister --network ic call bridge add_chain "(5:nat32, \"71B8D5F8b2d3e60CA0A4FfFb497Fe48DD961a44C\")"
dfx canister --network ic call bridge add_chain "(80001:nat32, \"22216796e65F3C786d853F818ca1fc0f661639C0\")"

dfx canister --network ic call bridge add_chain "(0:nat32, \"\")"

dfx canister --network ic call bridge create_pool "(\"$(dfx canister --network ic id usdt)\", 6:nat8)"
