import fs from 'fs';
import { ethers } from "hardhat";

export const getNonce = async function() {
    const addrs = await ethers.getSigners();
    let addr = addrs[0].address;
    let nonce = await ethers.provider.getTransactionCount(addr);
    console.log("nonce:", nonce);
    return nonce;
}

export const getChainId = function(network: string | number) {
    if(typeof network == "number") {
        return network;
    }
    let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
    return config.ChainIds[network];
}

export const updateConfig = function (network: string, contract: string, addr: string) {
    let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
    if(config.networks[network] == undefined) {
        config.networks[network] = {
            "Omnic": "",
            "Bridge": "",
            "Router": "",
            "FactoryPool": "",
            "USDT": "",
        };
    }
    config.networks[network][contract] = addr;
    fs.writeFileSync("config.json", JSON.stringify(config, null, "\t"));
    console.log(network, ":", config.networks[network]);
}

export const getContractAddr = function(network: string, contract: string) {
    let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
    if(config.networks[network] == undefined) {
        return null;
    }
    if(config.networks[network][contract] == undefined) {
        return null;
    }
    let res = config.networks[network][contract];
    if(res == "") {
        return null;
    }
    return res;
}

export const getProxyCanisterAddr = function() {
    let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
    return config.OmnicCanisterAddr;
}

export const getBridgeCanisterAddr = function() {
    let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
    return config.BridgeCanisterAddr;
}

export const getBridgeCanisterIdBytes = function() {
    let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
    return ethers.utils.hexZeroPad(config.BridgeCanisterIdBytes, 32);
}

export const abi_encode = function(abi: Array<string>, func: string, args: Array<string>) {
    let iface = new ethers.utils.Interface(abi);
    return iface.encodeFunctionData(func, args);
}

export const encodeCalldata = function(addr: string) {
    let abi = [
        "function initialize(address proxyCanisterAddr)"
    ];
    return abi_encode(abi, "initialize", [addr]);
}
