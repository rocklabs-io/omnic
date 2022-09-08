// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;
pragma abicoder v2;

// imports external
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/math/SafeMath.sol";

// imports internal
import "./Pool.sol";
import "./Router.sol";

import "./interfaces/IBridge.sol";
import "./interfaces/IOmnic.sol";

contract Bridge is IBridge, Ownable {
    using SafeMath for uint256;

    //----------------------------- Enums ----------------------------------------------
    enum OperationTypes {
        Invalid, // 0
        AddLiquidity, // 1
        Swap, // 2
        RemoveLiquidity // 3
    }

    //--------------------------- variables ----------------------------------------------

    IOmnic public immutable omnic;
    mapping(uint16 => bytes) public bridgeLookup;
    mapping(uint16 => mapping(uint8 => uint256)) public gasLookup;
    Router public immutable router;
    bool public useLayerZeroToken;

    //---------------------------- events -----------------------------------------------

    event SendMsg(uint8 msgType, uint64 nonce);

    //------------------------------- modifiers ------------------------------------------

    modifier onlyRouter() {
        require(msg.sender == address(router), "caller must be Router.");
        _;
    }

    constructor(address _omnic, address _router) {
        require(_omnic != address(0x0), "_omnic cannot be 0x0");
        require(_router != address(0x0), "_router cannot be 0x0");
        omnic = IOmnic(_omnic);
        router = Router(_router);
    }

    //----------------------------- router called  functions ------------------------------

    function handle() onlyRouter {

    }

    function _call(
        OperationTypes _type,
        uint16 _chainId,
        bytes memory _payload
    ) internal {
        // todo
        emit SendMsg(_type, nextNonce);
    }

    function renounceOwnership() public override onlyOwner {}
}