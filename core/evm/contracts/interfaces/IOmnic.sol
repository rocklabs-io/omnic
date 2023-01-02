// SPDX-License-Identifier: MIT

pragma solidity ^0.8.9;

interface IOmnic {
    function sendMessage(
        uint32 _dstChainId,
        bytes32 _recipientAddress,
        bytes memory _payload,
        address payable _refundAddress,
        address _erc20PaymentAddress
    ) external;

    function handleMessage(
        uint32 _srcChainId,
        bytes32 _srcSenderAddress,
        uint32 _nonce,
        bytes calldata payload
    ) external;
}
