// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

// XAppConnectionManager interface

interface IXAppConnectionManager {
    function omnic() external view returns (address);

    function isOmnicContract(address _omnic) external view returns (bool);

    function localChainId() external view returns (uint32);
}