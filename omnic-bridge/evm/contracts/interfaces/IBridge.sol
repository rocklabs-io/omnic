pragma solidity ^0.8.9;

interface IBridge {
    
    function swap() external;
    function addLiquidity() external;
    function removeLiquidity() external;
}