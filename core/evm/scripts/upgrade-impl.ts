import { ethers } from "hardhat";
import { getContractAddr, updateConfig } from "./helpers";
import fs from 'fs';
const hre = require("hardhat");
import { deployOmnicImpl } from "./deploy-impl";

// deploy Omnic implementataion contract
// and set the implementation address to UpgradeBeacon by calling UpgradeBeaconController

export const setOmnicImpl = async function (chain: string, implAddr: string) {
    let controller = await ethers.getContractAt(
        "UpgradeBeaconController",
        getContractAddr(chain, "UpgradeBeaconController")
    );
    
    console.log("call UpgradeBeaconController.upgrade...");
    let tx = await controller.upgrade(
        getContractAddr(chain, "UpgradeBeacon"),
        implAddr
    );
    console.log("txhash:", tx.hash);

    console.log("saving implementation to config...");
    updateConfig(chain, "Implementation", implAddr);
}

const main = async function () {
  let chain = hre.network.name;
  const upgrade = true;
  const impl_address = await deployOmnicImpl(chain, upgrade);
  await setOmnicImpl(chain, impl_address);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
