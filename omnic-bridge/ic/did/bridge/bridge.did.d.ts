import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export interface Pool {
  'shared_decimals' : number,
  'token' : Token,
  'src_pool_id' : number,
  'local_decimals' : number,
  'liquidity' : bigint,
  'src_chain' : number,
  'convert_rate' : bigint,
  'pool_address' : string,
}
export type Result = { 'Ok' : boolean } |
  { 'Err' : string };
export type Result_1 = { 'Ok' : Router } |
  { 'Err' : string };
export type Result_2 = { 'Ok' : Array<[number, Router]> } |
  { 'Err' : string };
export type Result_3 = { 'Ok' : Pool } |
  { 'Err' : string };
export type Result_4 = { 'Ok' : string } |
  { 'Err' : string };
export interface Router {
  'src_chain' : number,
  'pools' : Array<[number, Pool]>,
  'token_pool' : Array<[string, number]>,
  'bridge_addr' : string,
}
export interface Token {
  'decimals' : number,
  'name' : string,
  'address' : string,
  'symbol' : string,
}
export interface _SERVICE {
  'add_chain' : ActorMethod<[number, string], Result>,
  'add_owner' : ActorMethod<[Principal], Result>,
  'check_swap' : ActorMethod<[number, number, bigint], Result>,
  'create_pool' : ActorMethod<[string, number], Result>,
  'get_router' : ActorMethod<[number], Result_1>,
  'get_routers' : ActorMethod<[], Result_2>,
  'handle_message' : ActorMethod<
    [number, Array<number>, number, Array<number>],
    Result,
  >,
  'pool_by_token_address' : ActorMethod<[number, string], Result_3>,
  'remove_owner' : ActorMethod<[Principal], Result>,
  'set_canister_addr' : ActorMethod<[], Result_4>,
  'set_omnic' : ActorMethod<[Principal], Result>,
  'swap' : ActorMethod<[number, number, number, string, bigint], Result_4>,
}
