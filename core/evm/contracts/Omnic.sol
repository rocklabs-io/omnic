// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

// Omnic Crosschain message passing protocol core contract

contract Omnic {
    address public owner;
    address public omnicCanisterAddr;

    // Maximum bytes per message = 2 KiB
    // (somewhat arbitrarily set to begin)
    uint256 public constant MAX_MESSAGE_BODY_BYTES = 2 * 2**10;

    // domain => next available nonce for the domain
    mapping(uint32 => uint32) public nonces;

    // messages
    struct Message {
        uint32 origin;
        uint32 nonce;
        uint32 destination;
        bytes32 recipient;
        bytes message;
    }
    Message[] messages;

    event EnqueueMessage(
        bytes32 indexed messageHash,
        uint32 indexed destination,
        uint32 indexed nonce,
        bytes32 recepient,
        bytes message
    );

    event ProcessMessage(
        bytes32 indexed messageHash,
        uint32 indexed origin,
        uint32 indexed nonce,
        bytes32 recepient,
        bytes message
    );

    constructor() {
        owner = msg.sender;
    }

    function setOmnicCanisterAddr(address addr) public {
        require(msg.sender == owner);
        omnicCanisterAddr = addr;
    }

    // TODO: implementation
    // // simplify this: https://github.com/nomad-xyz/monorepo/blob/main/packages/contracts-core/contracts/Home.sol#L175
    // function enqueueMessage(
    //     uint32 destination,
    //     bytes32 recipient,
    //     bytes memory data
    // ) public {
    //     require(data.length <= MAX_MESSAGE_BODY_BYTES, "msg too long");
    //     // get the next nonce for the destination domain, then increment it
    //     uint32 _nonce = nonces[destination];
    //     nonces[destination] = _nonce + 1;

    // }

    // // only omnic canister can call this func
    // function processMessage() public {
    //     require(msg.sender == omnicCanisterAddr);
        
    // }
}
