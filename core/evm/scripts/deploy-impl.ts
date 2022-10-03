import { ethers } from "hardhat";
import { getContractAddr, updateConfig } from "./helpers";
import fs from 'fs';
const hre = require("hardhat");

// deploy Omnic implementataion contract
// and set the implementation address to UpgradeBeacon by calling UpgradeBeaconController

export const deployOmnicImpl = async function (chain: string, upgrade: boolean) {
  const Omnic = await ethers.getContractFactory("Omnic");

  // deploy UpgradeBeaconController
  const omnicAddr = getContractAddr(chain, "Implementation");
  let omnic;
  // if it is upgrade, redeploy the implementation even though we found an existing implemenataion
  // otherwise, just return the deployed one
  if(omnicAddr == null || upgrade == true) {
    console.log("deploying Omnic implemenataion...");
    omnic = await Omnic.deploy();

    await omnic.deployed();
    console.log("chain: ", chain, "Omnic implementation deployed to:", omnic.address);
    // recording omnic contract address
    updateConfig(chain, "Implementation", omnic.address);
  } else {
    console.log("found deployed Omnic implementation:", omnicAddr);
    omnic = ethers.getContractAt("Omnic", omnicAddr);
  }
  return omnic;
}

export const setOmnicImpl = async function (chain: string, implAddr: string) {

}

const main = async function () {
  let chain = hre.network.name;
  const upgrade = false;
  const impl  = await deployOmnicImpl(chain, upgrade);
  await setOmnicImpl(chain, impl.address);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
