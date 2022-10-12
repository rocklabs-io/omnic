import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export interface Metadata {
  'fee' : bigint,
  'decimals' : number,
  'owner' : Principal,
  'logo' : string,
  'name' : string,
  'totalSupply' : bigint,
  'symbol' : string,
}
export type Operation = { 'transferFrom' : null } |
  { 'burn' : null } |
  { 'mint' : null } |
  { 'approve' : null } |
  { 'transfer' : null };
export type Time = bigint;
export interface Token {
  'allowance' : ActorMethod<[Principal, Principal], bigint>,
  'approve' : ActorMethod<[Principal, bigint], TxReceipt>,
  'balanceOf' : ActorMethod<[Principal], bigint>,
  'burn' : ActorMethod<[bigint], TxReceipt>,
  'burnFrom' : ActorMethod<[Principal, bigint], TxReceipt>,
  'decimals' : ActorMethod<[], number>,
  'getAllowanceSize' : ActorMethod<[], bigint>,
  'getHolders' : ActorMethod<[bigint, bigint], Array<[Principal, bigint]>>,
  'getMetadata' : ActorMethod<[], Metadata>,
  'getTokenFee' : ActorMethod<[], bigint>,
  'getTokenInfo' : ActorMethod<[], TokenInfo>,
  'getTransaction' : ActorMethod<[bigint], TxRecord>,
  'getTransactions' : ActorMethod<[bigint, bigint], Array<TxRecord>>,
  'getUserApprovals' : ActorMethod<[Principal], Array<[Principal, bigint]>>,
  'getUserTransactionAmount' : ActorMethod<[Principal], bigint>,
  'getUserTransactions' : ActorMethod<
    [Principal, bigint, bigint],
    Array<TxRecord>,
  >,
  'historySize' : ActorMethod<[], bigint>,
  'logo' : ActorMethod<[], string>,
  'mint' : ActorMethod<[Principal, bigint], TxReceipt>,
  'name' : ActorMethod<[], string>,
  'setFee' : ActorMethod<[bigint], undefined>,
  'setFeeTo' : ActorMethod<[Principal], undefined>,
  'setLogo' : ActorMethod<[string], undefined>,
  'setOwner' : ActorMethod<[Principal], undefined>,
  'symbol' : ActorMethod<[], string>,
  'totalSupply' : ActorMethod<[], bigint>,
  'transfer' : ActorMethod<[Principal, bigint], TxReceipt>,
  'transferFrom' : ActorMethod<[Principal, Principal, bigint], TxReceipt>,
}
export interface TokenInfo {
  'holderNumber' : bigint,
  'deployTime' : Time,
  'metadata' : Metadata,
  'historySize' : bigint,
  'cycles' : bigint,
  'feeTo' : Principal,
}
export type TransactionStatus = { 'inprogress' : null } |
  { 'failed' : null } |
  { 'succeeded' : null };
export type TxReceipt = { 'Ok' : bigint } |
  {
    'Err' : { 'InsufficientAllowance' : null } |
      { 'InsufficientBalance' : null } |
      { 'ErrorOperationStyle' : null } |
      { 'Unauthorized' : null } |
      { 'LedgerTrap' : null } |
      { 'ErrorTo' : null } |
      { 'Other' : string } |
      { 'BlockUsed' : null } |
      { 'AmountTooSmall' : null }
  };
export interface TxRecord {
  'op' : Operation,
  'to' : Principal,
  'fee' : bigint,
  'status' : TransactionStatus,
  'from' : Principal,
  'timestamp' : Time,
  'caller' : [] | [Principal],
  'index' : bigint,
  'amount' : bigint,
}
export interface _SERVICE extends Token {}
