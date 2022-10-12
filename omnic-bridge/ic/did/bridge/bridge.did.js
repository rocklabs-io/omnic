export const idlFactory = ({ IDL }) => {
  const Result = IDL.Variant({ 'Ok' : IDL.Bool, 'Err' : IDL.Text });
  const Token = IDL.Record({
    'decimals' : IDL.Nat8,
    'name' : IDL.Text,
    'address' : IDL.Text,
    'symbol' : IDL.Text,
  });
  const Pool = IDL.Record({
    'shared_decimals' : IDL.Nat8,
    'token' : Token,
    'src_pool_id' : IDL.Nat32,
    'local_decimals' : IDL.Nat8,
    'liquidity' : IDL.Nat,
    'src_chain' : IDL.Nat32,
    'convert_rate' : IDL.Nat,
    'pool_address' : IDL.Text,
  });
  const Router = IDL.Record({
    'src_chain' : IDL.Nat32,
    'pools' : IDL.Vec(IDL.Tuple(IDL.Nat32, Pool)),
    'token_pool' : IDL.Vec(IDL.Tuple(IDL.Text, IDL.Nat32)),
    'bridge_addr' : IDL.Text,
  });
  const Result_1 = IDL.Variant({ 'Ok' : Router, 'Err' : IDL.Text });
  const Result_2 = IDL.Variant({
    'Ok' : IDL.Vec(IDL.Tuple(IDL.Nat32, Router)),
    'Err' : IDL.Text,
  });
  const Result_3 = IDL.Variant({ 'Ok' : Pool, 'Err' : IDL.Text });
  const Result_4 = IDL.Variant({ 'Ok' : IDL.Text, 'Err' : IDL.Text });
  return IDL.Service({
    'add_chain' : IDL.Func([IDL.Nat32, IDL.Text], [Result], []),
    'add_owner' : IDL.Func([IDL.Principal], [Result], []),
    'check_swap' : IDL.Func(
        [IDL.Nat32, IDL.Nat32, IDL.Nat64],
        [Result],
        ['query'],
      ),
    'create_pool' : IDL.Func([IDL.Text, IDL.Nat8], [Result], []),
    'get_router' : IDL.Func([IDL.Nat32], [Result_1], ['query']),
    'get_routers' : IDL.Func([], [Result_2], ['query']),
    'handle_message' : IDL.Func(
        [IDL.Nat32, IDL.Vec(IDL.Nat8), IDL.Nat32, IDL.Vec(IDL.Nat8)],
        [Result],
        [],
      ),
    'pool_by_token_address' : IDL.Func(
        [IDL.Nat32, IDL.Text],
        [Result_3],
        ['query'],
      ),
    'remove_owner' : IDL.Func([IDL.Principal], [Result], []),
    'set_canister_addr' : IDL.Func([], [Result_4], []),
    'set_omnic' : IDL.Func([IDL.Principal], [Result], []),
    'swap' : IDL.Func(
        [IDL.Nat32, IDL.Nat32, IDL.Nat32, IDL.Text, IDL.Nat64],
        [Result_4],
        [],
      ),
  });
};
export const init = ({ IDL }) => { return []; };
