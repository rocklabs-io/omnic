// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

//internal
import {QueueLib} from "./libs/Queue.sol";
import {QueueManager} from "./utils/QueueManager.sol";
import {MerkleLib} from "./libs/Merkle.sol";
import {Types} from "./libs/Types.sol";
import {TypeCasts} from "./utils/Utils.sol";
import {IOmnicReciver} from "./interfaces/IOmnicReciver.sol";

//external
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";


contract OmnicBase is Initializable, OwnableUpgradeable {

    // ============ Constants ============
    uint32 public chainId;

    // gap for upgrade safety
    uint256[49] private __GAP;

    
    // ============== Start ===============
    constructor() {
        chainId = uint32(block.chainid);
    }

    function __OmnicBase_initialize() internal initializer {
        __Ownable_init();
        chainId = uint32(block.chainid);   
    }
}
