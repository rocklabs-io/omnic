// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

// Omnic interface

interface IOmnic {
    function enqueueMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory payload
    ) external;
}