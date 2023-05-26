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

  const proxyCanisterAddr = "0x012709e1293b5fb2476a0d6a6011a4944d97bdbf"
  // const proxyCanisterAddr = "0x385F27cb1b920cC6170cbF62740aE1B4A707cFd0"
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
