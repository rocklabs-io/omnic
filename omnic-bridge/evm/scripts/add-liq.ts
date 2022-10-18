import { BigNumber } from "ethers";
import { ethers } from "hardhat";
import { getContractAddr, getChainId } from "./helpers";
const hre = require("hardhat");

export const approveToken = async function(
  chain: string,
  tokenSymbol: string,
  spender: string,
  amount: number
  ) {
  let tokenAddr = getContractAddr(chain, tokenSymbol);
  const token = await ethers.getContractAt("ERC20", tokenAddr);
  
  const addrs = await ethers.getSigners();
  let caller = addrs[0].address;
  console.log("owner:", caller);
  let allowance = (await token.allowance(caller, spender)).toNumber();
  console.log("allowance:", allowance);
  if(allowance == 0) {
    console.log("approving...");
    let tx = await token.approve(spender, amount);
    console.log("approve tx:", tx.hash);
    let res = await tx.wait();
    // console.log("res:", res);
  } else {
    console.log("already approved, allowance:", allowance);
  }
}

export const addLiquidity = async function (
  chain: string, 
  tokenSymbol: string,
  amount: number, 
  to: string,
  ) {
  // const token = await ethers.getContractAt("ERC20", tokenAddr);
  const router = await ethers.getContractAt("Router", getContractAddr(chain, "Router"));
  const factory = await ethers.getContractAt("FactoryPool", getContractAddr(chain, "FactoryPool"));

  await approveToken(chain, tokenSymbol, router.address, 1_000_000_000_000_000);

  
  const pool_id = await factory.getPoolId(getContractAddr(chain, tokenSymbol));
  
  let tx = await router.addLiquidity(
    pool_id,
    amount,
    to
    );
  console.log("addLiquidity tx:", tx.hash);
}

// send USDT to IC
const main = async function () {
  let chain = hre.network.name;
  let amount = 100000_000_000;
  const addrs = await ethers.getSigners();
  let to = addrs[0].address;
  await addLiquidity(chain, "USDT", amount, to);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
