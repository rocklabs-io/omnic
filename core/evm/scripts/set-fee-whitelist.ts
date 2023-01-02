import { ethers } from "hardhat";
import { getContractAddr, updateConfig, getProxyCanisterAddr, encodeCalldata } from "./helpers";
const hre = require("hardhat");

// deploy upgrade related contracts:
// 1. UpgradeBeaconController: controls UpgradeBeacon
// 2. UpgradeBeacon: stores the implementation address
// 3. UpgradeBeaconProxy: the proxy contract that users interact with

const setWhitelist = async function (chain: string) {
  const UpgradeBeaconProxy = await ethers.getContractFactory("UpgradeBeaconProxy");

  // deploy UpgradeBeaconProxy
  // deploy UpgradeBeacon
  const proxyAddr = getContractAddr(chain, "UpgradeBeaconProxy");
  console.log("found deployed UpgradeBeaconProxy:", proxyAddr);
  let proxy = ethers.getContractAt("UpgradeBeaconProxy", proxyAddr);

  // set fee whitelist
  let bridgeAddr = getContractAddr(chain, "Bridge");
  let tx = await proxy.setWhitelist(bridgeAddr, true);
  console.log("setWhitelist tx:", tx);
}

const main = async function() {
  let chain = hre.network.name;
  await setWhitelist(chain);
  console.log("All done!")
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
