pragma solidity ^0.8.12;

library BirdgeManager {
 
    // ============ Enums ============
    enum OperationTypes {
        Invalid,
        Transfer
    }

    function isTransfer(uint32 _operation) internal pure returns (bool) {
        return _operation == uint32(OperationTypes.Transfer);
    }

    function getDetailsHash(
        string memory _name,
        string memory _symbol,
        uint8 _decimals
    ) internal pure returns (bytes32) {
        return
            keccak256(
                abi.encodePacked(
                    bytes(_name).length,
                    _name,
                    bytes(_symbol).length,
                    _symbol,
                    _decimals
                )
            );
    }



}