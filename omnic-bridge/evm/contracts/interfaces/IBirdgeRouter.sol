// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;
pragma abicoder v2;

interface IBirdgeRouter {

    function addLiquidity(
        uint256 _poolId,
        uint256 _amountLD,
        address _to
    ) external;

    function swap(
        uint16 _dstChainId,
        uint256 _srcPoolId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        uint256 _minAmountLD,
        bytes calldata _to,
        bytes calldata _payload
    ) external payable;


    function removeLiquidity(
        uint16 _srcPoolId,
        uint256 _amountLP,
        address _to
    ) external returns (uint256);

}