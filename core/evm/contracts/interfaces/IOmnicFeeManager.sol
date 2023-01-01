// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.9;

interface IOmnicFeeManager {
    /**
     * @notice call it to get protocol fee
     * @param payInERC20 if using ERC20 for fee
     * @param msgLength message length
     */
    function getFees(bool payInERC20, uint256 msgLength)
        external
        view
        returns (uint256);
}