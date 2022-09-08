// SPDX-License-Identifier: MIT

pragma solidity ^0.8.9;

interface IOmnic {
    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bool _waitOptimistic, // customized
        bytes memory _payload
    ) external;

    function handleMessage(bytes memory _message) external view returns (bool);
}
