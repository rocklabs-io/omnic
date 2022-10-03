import { ethers } from "hardhat";
import { getContractAddr, updateConfig, getProxyCanisterAddr, encodeCalldata } from "./helpers";
const hre = require("hardhat");

// deploy upgrade related contracts:
// 1. UpgradeBeaconController: controls UpgradeBeacon
// 2. UpgradeBeacon: stores the implementation address
// 3. UpgradeBeaconProxy: the proxy contract that users interact with

const deployProxy = async function (chain: string) {
  const UpgradeBeaconController = await ethers.getContractFactory("UpgradeBeaconController");
  const UpgradeBeacon = await ethers.getContractFactory("UpgradeBeacon");
  const UpgradeBeaconProxy = await ethers.getContractFactory("UpgradeBeaconProxy");

  // deploy UpgradeBeaconController
  const controllerAddr = getContractAddr(chain, "UpgradeBeaconController");
  let controller;
  if(controllerAddr == null) {
    console.log("deploying UpgradeBeaconController...");
    controller = await UpgradeBeaconController.deploy();

    await controller.deployed();
    console.log("chain: ", chain, "UpgradeBeaconController deployed to:", controller.address);
    // recording omnic contract address
    updateConfig(chain, "UpgradeBeaconController", controller.address);
  } else {
    console.log("found deployed UpgradeBeaconController:", controllerAddr);
    controller = await ethers.getContractAt("UpgradeBeaconController", controllerAddr);
  }

  const implAddr = getContractAddr(chain, "Implementation");
  if(implAddr == null) {
    console.log("Implementation not found! Please deploy an initial implemetation first!");
    return null;
  }

  // deploy UpgradeBeacon
  const beaconAddr = getContractAddr(chain, "UpgradeBeacon");
  let beacon;
  if(beaconAddr == null) {
    console.log("deploying UpgradeBeacon...");
    beacon = await UpgradeBeacon.deploy(implAddr, controller.address);

    await beacon.deployed();
    console.log("chain: ", chain, "UpgradeBeacon deployed to:", beacon.address);
    // recording omnic contract address
    updateConfig(chain, "UpgradeBeacon", beacon.address);
  } else {
    console.log("found deployed UpgradeBeacon:", beaconAddr);
    beacon = await ethers.getContractAt("UpgradeBeacon", beaconAddr);
  }

  // deploy UpgradeBeaconProxy
  // deploy UpgradeBeacon
  const proxyAddr = getContractAddr(chain, "UpgradeBeaconProxy");
  let proxy;
  if(proxyAddr == null) {
    console.log("deploying UpgradeBeaconProxy...");
    const initCalldata = encodeCalldata(getProxyCanisterAddr());
    proxy = await UpgradeBeaconProxy.deploy(beacon.address, initCalldata);

    await proxy.deployed();
    console.log("chain: ", chain, "UpgradeBeaconProxy deployed to:", proxy.address);
    // recording omnic contract address
    updateConfig(chain, "UpgradeBeaconProxy", proxy.address);
  } else {
    console.log("found deployed UpgradeBeaconProxy:", proxyAddr);
    controller = ethers.getContractAt("UpgradeBeaconProxy", proxyAddr);
  }
}

const main = async function() {
  let chain = hre.network.name;
  await deployProxy(chain);
  console.log("All done!")
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
