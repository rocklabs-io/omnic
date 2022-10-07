import { ethers } from "hardhat";
import { getContractAddr, getChainId } from "./helpers";
const hre = require("hardhat");

export const swap = async function (
  chain: string, 
  tokenSymbol: string,
  destination: string,
  amount: number, 
  recipient: string,
  ) {
  let tokenAddr = getContractAddr(chain, tokenSymbol);
  const token = await ethers.getContractAt("ERC20", tokenAddr);
  const router = await ethers.getContractAt("BridgeRouter", getContractAddr(chain, "BridgeRouter"));
  const factory = await ethers.getContractAt("FactoryPool", getContractAddr(chain, "FactoryPool"));

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
  let amount = 1_000_000;
  // pid: 7bv5o-swpxq-yx3sg-eirhj-rn7tm-7fnh5-pnovl-um577-4qatm-nfesf-iae
  let recipient = "cfbc317dc8c4444e98b7f367cad3f5ed75574677ffe4013634a4915002";
  let recipient_pad = ethers.utils.hexZeroPad(recipient, 32);
  await swap(chain, "USDT", destination, amount, recipient_pad);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
