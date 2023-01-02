import { ethers } from "hardhat";
import { getContractAddr, updateConfig } from "./helpers";
import fs from 'fs';
const hre = require("hardhat");

// deploy Omnic implementataion contract
// and set the implementation address to UpgradeBeacon by calling UpgradeBeaconController

export const deployFeeManager = async function (chain: string) {
  const FeeManager = await ethers.getContractFactory("OmnicFeeManager");

  const feeAddr = getContractAddr(chain, "OmnicFeeManager");
  let feeManager;
  let nativeBaseFee = 1 * 10**15; // 0.001 eth
  let nativeFeePerByte = 0;
  let erc20FeeToken = getContractAddr(chain, 'USDT');
  let erc20BaseFee = 0;
  let erc20FeePerByte = 0;
  if(feeAddr == null) {
    console.log("deploying OmnicFeeManager...");
    feeManager = await FeeManager.deploy(
      true,
      true,
      erc20FeeToken,
      nativeBaseFee,
      nativeFeePerByte,
    );

    await feeManager.deployed();
    console.log("chain: ", chain, "OmnicFeeManager deployed to:", feeManager.address);
    updateConfig(chain, "OmnicFeeManager", feeManager.address);
  } else {
    console.log("found deployed OmnicFeeManager:", feeAddr);
    feeManager = await ethers.getContractAt("OmnicFeeManager", feeAddr);
  }
  return feeManager;
}

const main = async function () {
  let chain = hre.network.name;
  await deployFeeManager(chain);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
