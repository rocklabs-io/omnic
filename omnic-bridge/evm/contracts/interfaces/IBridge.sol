// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;

interface IBridge {
    function swap(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint16 _dstChainId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        bytes32 _to,
        bool _waitOptimistic
    ) external;

    function addLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        bool _waitOptimistic,
        uint256 _amount
    ) external;

    function removeLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        bool _waitOptimistic,
        uint256 _amount
    ) external;
}
