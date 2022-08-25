// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import {QueueLib} from "./libs/Queue.sol";
import {QueueManager} from "./Queue.sol";
import {Types} from "./libs/Types.sol";
import {IMessageRecipient} from "./interfaces/IMessageRecipient.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";
// Demo app contract to receive message from omnic core contract

contract Demo is Ownable, QueueManager {

    // only omnic contract can call this func
    function handleMessage(

    ) public returns (bool success) {

    }
}
