import { ethers } from "hardhat";
import { getContractAddr, updateConfig } from "./helpers";
import fs from 'fs';
const hre = require("hardhat");

// deploy Omnic implementataion contract
// and set the implementation address to UpgradeBeacon by calling UpgradeBeaconController

export const setDemoOmnicAddr = async function (chain: string) {
  const DemoApp = await ethers.getContractFactory("DemoApp");

  const omnicAddr = getContractAddr(chain, "UpgradeBeaconProxy");
  const demoAddr = getContractAddr(chain, "Demo");
  let demo;
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
  let tx = await demo.setOmnicContractAddr(omnicAddr);
  console.log("txhash:", tx.hash);
}

const main = async function () {
  let chain = hre.network.name;
  await setDemoOmnicAddr(chain);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
