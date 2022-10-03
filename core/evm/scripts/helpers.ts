import fs from 'fs';

export const updateConfig = function (network: string, contract: string, addr: string) {
    let config = JSON.parse(fs.readFileSync('./config.json', 'utf-8'));
    if(config.networks[network] == undefined) {
        config.networks[network] = {
            "UpgradeBeaconController": "",
            "UpgradeBeacon": "",
            "UpgradeBeaconProxy": "",
            "Implementation": "",
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

