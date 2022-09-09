// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;

interface IBridge {
    function swap(
        uint16 _dstChainId,
        bytes32 _dstRecipientAddress,
        bool _waitOptimistic,
        bytes memory _payload
    ) external;

    function addLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint16 _dstChainId,
        bytes32 _dstRecipientAddress,
        bool _waitOptimistic,
        uint256 _amount
    ) external;

    function removeLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint16 _dstChainId,
        bytes32 _dstRecipientAddress,
        bool _waitOptimistic,
        uint256 _amount
    ) external;
}
