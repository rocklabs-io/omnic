// SPDX-License-Identifier: MIT

pragma solidity ^0.8.9;

interface IOmnic {
    
    function sendMessage() external view returns (bool);

    function handleMessage() external view returns (bool);

}