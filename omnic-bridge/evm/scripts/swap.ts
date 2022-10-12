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
  } else {
    console.log("already approved, allowance:", allowance);
  }
}

export const swap = async function (
  chain: string, 
  tokenSymbol: string,
  destination: string,
  amount: number, 
  recipient: string,
  ) {
  let tokenAddr = getContractAddr(chain, tokenSymbol);
  // const token = await ethers.getContractAt("ERC20", tokenAddr);
  const router = await ethers.getContractAt("Router", getContractAddr(chain, "Router"));
  const factory = await ethers.getContractAt("FactoryPool", getContractAddr(chain, "FactoryPool"));

  await approveToken(chain, tokenSymbol, router.address, 1_000_000_000_000_000);

  /*
    uint16 _dstChainId,
    uint256 _srcPoolId,
    uint256 _dstPoolId,
    uint256 _amountLD,
    uint256 _minAmountLD,
    bytes32 _to
  */
 // How to get pool id with token address?
  let dst_pool_id;
  if(destination == "ic") {
    // get pool id from ic
    dst_pool_id = 0;
  } else {
    const dst_factory = await ethers.getContractAt("FactoryPool", getContractAddr(destination, "FactoryPool"));
    dst_pool_id = await dst_factory.getPoolId(getContractAddr(destination, tokenSymbol));
  }
  
  let tx = await router.swap(
    getChainId(destination),
    await factory.getPoolId(tokenAddr),
    dst_pool_id,
    amount,
    amount,
    recipient
    );
  console.log("swap tx:", tx.hash);
}

// send USDT to IC
const main = async function () {
  let chain = hre.network.name;
  let destination = "ic";
  let amount = 100_000_000;
  // pid: f2bzr-sdq5g-orzxi-hs4u2-ohqvg-i3ln7-x7hny-q7xpm-g4d6t-acczq-2qe
  let recipient = "0x70e99d1cdd079729a71e153236b6feff3b710fddec3707e98042cc3502";
  let recipient_pad = ethers.utils.hexZeroPad(recipient, 32);
  await swap(chain, "USDT", destination, amount, recipient_pad);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
