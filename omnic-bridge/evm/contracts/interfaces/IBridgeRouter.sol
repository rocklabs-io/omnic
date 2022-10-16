// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;
pragma abicoder v2;

interface IBridgeRouter {
    function addLiquidity(
        uint256 _poolId,
        uint256 _amountLD,
        address _to
    ) external;

    function swap(
        uint32 _dstChainId,
        uint256 _srcPoolId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        uint256 _minAmountLD,
        bytes32 _to
    ) external;

    function removeLiquidity(
        uint32 _srcPoolId,
        uint256 _amountLP,
        address _to
    ) external;

    function handleSwap(
        uint256 _nonce,
        uint32 _dstChainId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        bytes32 _to
    ) external;

    function revertFailedSwap(
        uint32 _srcChainId,
        uint256 _srcPoolId,
        uint256 _amountLD,
        bytes32 _to
    ) external;
}
