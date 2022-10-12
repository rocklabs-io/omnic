// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;

interface IBridge {
    function swap(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint16 _dstChainId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        bytes32 _to
    ) external;

    function addLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint256 _amount
    ) external;

    function removeLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint256 _amount
    ) external;

    function createPool (
        uint256 _poolId,
        address _poolAddr,
        address _tokenAddr,
        uint8 _sharedDecimals,
        uint8 _localDecimals,
        string memory _name,
        string memory _symbol
    ) external;
}
