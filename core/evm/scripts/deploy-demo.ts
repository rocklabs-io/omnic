import { ethers } from "hardhat";
import { getContractAddr, updateConfig } from "./helpers";
import fs from 'fs';
const hre = require("hardhat");

// deploy Omnic implementataion contract
// and set the implementation address to UpgradeBeacon by calling UpgradeBeaconController

export const deployDemo = async function (chain: string) {
  const DemoApp = await ethers.getContractFactory("DemoApp");

  const omnicAddr = getContractAddr(chain, "Implementation");
  const demoAddr = getContractAddr(chain, "Demo");
  let demo;
  // if it is upgrade, redeploy the implementation even though we found an existing implemenataion
  // otherwise, just return the deployed one
  if(demoAddr == null) {
    console.log("deploying Demo App...");
    demo = await DemoApp.deploy(omnicAddr);

    await demo.deployed();
    console.log("chain: ", chain, "DemoApp deployed to:", demo.address);
    updateConfig(chain, "Demo", demo.address);
  } else {
    console.log("found deployed DempApp:", demoAddr);
    demo = await ethers.getContractAt("DemoApp", demoAddr);
  }
  return demo;
}

const main = async function () {
  let chain = hre.network.name;
  const upgrade = false;
  await deployDemo(chain);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});