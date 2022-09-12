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
import {addressToBytes32, bytes32ToAddress} from "./utils/TypeCasts.sol";

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
    uint256 public nonce;
    IOmnic public immutable omnic;
    Router public immutable router;
    uint16 public chainIdIC;
    address public bridgeOnIC;

    //---------------------------- events -----------------------------------------------

    event SendMsg(uint8 msgType, uint64 nonce);
    event UdpateBridgeOnIC(address oldAddrss, address newAddrss);


    //------------------------------- modifiers & constructor------------------------------------------

    modifier onlyRouter() {
        require(msg.sender == address(router), "caller must be Router.");
        _;
    }

    modifier onlyBridgeOnIC() {
        require(
            msg.sender == address(bridgeOnIC),
            "Bridge: caller must be IC Bridge."
        );
        _;
    }

    constructor(
        address _omnic,
        address _router,
        uint16 _chainIdIC,
        address _bridgeOnIc
    ) {
        require(_omnic != address(0x0), "_omnic cannot be 0x0");
        require(_router != address(0x0), "_router cannot be 0x0");
        require(_bridgeOnIc != address(0x0), "_bridgeOnIc cannot be 0x0");
        omnic = IOmnic(_omnic);
        router = Router(_router);
        chainIdIC = _chainIdIC;
        bridgeOnIC = _bridgeOnIc;
    }

    //----------------------------- router called  functions ------------------------------

    function handle(bytes memory _payload)
        external
        view
        onlyBridgeOnIC
        returns (bool)
    {
        uint8 t;
        assembly {
            t := mload(add(_payload, 32))
        }
        if (t == uint8(OperationTypes.Swap)) {
            //decode data from bridge on IC
            (
                ,
                uint16 _dstChainId,
                uint256 _dstPoolId,
                uint256 _amountLD,
                bytes32 _to
            ) = abi.decode(_payload, uint8, uint16, uint256, uint256, bytes32);
            router.handleSwap(++nonce, _dstChainId, _dstPoolId, _amountLD, _to);
        }
        return true;
    }

    function revertFailedSwap(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint256 _amountLD,
        bytes32 _to
    ) external view onlyBridgeOnIC returns (bool) {
        router.revertFailedSwap();
        return true;
    }

    //----------------------------- router called  functions ------------------------------
    function swap(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint16 _dstChainId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        bytes32 _to,
        bool _waitOptimistic
    ) external override onlyRouter {
        bytes memory _payload = abi.encode(
            uint8(OperationTypes.Swap),
            _srcChainId,
            _srcPoolId,
            _dstChainId,
            _dstPoolId,
            _amountLD,
            _to
        );
        _send(OperationTypes.Swap, _waitOptimistic, _payload);
    }

    function addLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        bool _waitOptimistic,
        uint256 _amount
    ) external override onlyRouter {
        bytes memory _payload = abi.encode(
            uint8(OperationTypes.AddLiquidity),
            _srcChainId,
            _srcPoolId,
            _amount
        );
        _send(OperationTypes.AddLiquidity, _waitOptimistic, _payload);
    }

    function removeLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        bool _waitOptimistic,
        uint256 _amount
    ) external override onlyRouter {
        bytes memory _payload = abi.encode(
            uint8(OperationTypes.RemoveLiquidity),
            _srcChainId,
            _srcPoolId,
            _amount
        );
        _send(OperationTypes.RemoveLiquidity, _waitOptimistic, _payload);
    }

    //--------------------------- set functions------------------------------------------------

    function setBridgeOnIC(address _newAddress) public onlyOwner {
        require(_newAddress != address(0x0), "address cannot be 0x0");
        address oldAddress = bridgeOnIC;
        bridgeOnIC = _newAddress;
        emit UdpateBridgeOnIC(oldAddrss, _newAddrss);
    }

    //----------------------------- internal  functions ------------------------------
    function _send(
        OperationTypes _t,
        bool _waitOptimistic,
        bytes memory _payload
    ) internal {
        uint256 _nonce = nonce++;
        omnic.sendMessage(
            chainIdIC,
            addressToBytes32(bridgeOnIC),
            _waitOptimistic,
            _payload
        );
        emit SendMsg(_t, _nonce);
    }

    function renounceOwnership() public override onlyOwner {}
}
