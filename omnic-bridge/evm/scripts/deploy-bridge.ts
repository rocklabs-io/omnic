import { ethers } from "hardhat";
import { getContractAddr, updateConfig, getBridgeCanisterAddr } from "./helpers";
const hre = require("hardhat");

// Router -> Bridge -> FactoryPool -> call Router.setBridgeAndFactory
export const deployBridge = async function (chain: string) {
  const Router = await ethers.getContractFactory("Router");
  const Bridge = await ethers.getContractFactory("Bridge");
  const FactoryPool = await ethers.getContractFactory("FactoryPool");

  const routerAddr = getContractAddr(chain, "Router");
  let router;
  if(routerAddr == null) {
    console.log("deploying Router...");
    router = await Router.deploy();

    await router.deployed();
    console.log("chain: ", chain, "Router deployed to:", router.address);
    updateConfig(chain, "Router", router.address);
  } else {
    console.log("found deployed Router:", routerAddr);
    router = await ethers.getContractAt("Router", routerAddr);
  }

  const bridgeAddr = getContractAddr(chain, "Bridge");
  let bridge;
  if(bridgeAddr == null) {
    console.log("deploying Bridge...");
    let omnicAddr = getContractAddr(chain, "Omnic");
    bridge = await Bridge.deploy(omnicAddr, router.address, getBridgeCanisterAddr());

    await bridge.deployed();
    console.log("chain: ", chain, "Bridge deployed to:", bridge.address);
    updateConfig(chain, "Bridge", bridge.address);
  } else {
    console.log("found deployed Bridge:", bridgeAddr);
    bridge = await ethers.getContractAt("Bridge", bridgeAddr);
  }

  const factoryAddr = getContractAddr(chain, "FactoryPool");
  let factory;
  if(factoryAddr == null) {
    console.log("deploying FactoryPool...");
    factory = await FactoryPool.deploy(router.address);

    await factory.deployed();
    console.log("chain: ", chain, "FactoryPool deployed to:", factory.address);
    updateConfig(chain, "FactoryPool", factory.address);
  } else {
    console.log("found deployed FactoryPool:", factoryAddr);
    factory = await ethers.getContractAt("FactoryPool", factoryAddr);
  }

  // setBridgeAndFactory
  let tx = await router.setBridgeAndFactory(bridge.address, factory.address);
  console.log("BridgeRouter.setBridgeAndFactory tx:", tx.hash);
}

const main = async function () {
  let chain = hre.network.name;
  const upgrade = false;
  await deployBridge(chain);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
