import { ethers } from "hardhat";
import { getContractAddr } from "./helpers";
const hre = require("hardhat");


const deployProxy = async function (chain: string) {

  // const omnicAddr = getContractAddr(chain, "Implementation");
  // console.log("found deployed Omnic implementation:", omnicAddr);
  // let omnic = await ethers.getContractAt("Omnic", omnicAddr);

  const proxyAddr = getContractAddr(chain, "UpgradeBeaconProxy");
  console.log("found deployed UpgradeBeaconProxy:", proxyAddr);
  let proxy = await ethers.getContractAt("Omnic", proxyAddr);

  const proxyCanisterAddr = "0xF6C6FC3A0b3Bf682E17e6f45f4F4721e84A8ec70"
  let old_val = await proxy.omnicProxyCanisterAddr()
  console.log("get proxy address:", old_val)
  let result = await proxy.setOmnicCanisterAddr(proxyCanisterAddr)
  console.log("set omnic cainster address success, hash: " + result.hash)
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
