// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;
pragma abicoder v2;

// imports external
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";

// imports internal
import "./Pool.sol";
import "./Router.sol";

import "./interfaces/IBridge.sol";
import "./interfaces/IOmnic.sol";
import {TypeCasts} from "./utils/TypeCasts.sol";

contract Bridge is IBridge, Ownable {
    using SafeMath for uint256;

    //----------------------------- Enums ----------------------------------------------
    enum OperationTypes {
        Invalid, // 0
        AddLiquidity, // 1
        Swap, // 2
        RemoveLiquidity, // 3
        CreatePool // 4
    }

    //--------------------------- variables ----------------------------------------------
    uint256 public nonce;
    IOmnic public immutable omnic;
    Router public immutable router;
    uint16 public immutable chainIdIC = 0;
    address public bridgeCanister;

    //---------------------------- events -----------------------------------------------

    event SendMsg(OperationTypes msgType, uint256 nonce);
    event UdpateBridgeCanister(address oldAddress, address newAddress);


    //------------------------------- modifiers & constructor------------------------------------------

    modifier onlyRouter() {
        require(msg.sender == address(router), "caller must be Router.");
        _;
    }

    modifier onlyBridgeCanister() {
        require(
            msg.sender == address(bridgeCanister),
            "Bridge: caller must be IC Bridge canister"
        );
        _;
    }

    constructor(
        address _omnic,
        address _router,
        address _bridgeCanister
    ) {
        require(_omnic != address(0x0), "_omnic cannot be 0x0");
        require(_router != address(0x0), "_router cannot be 0x0");
        require(_bridgeCanister != address(0x0), "_bridgeOnIc cannot be 0x0");
        omnic = IOmnic(_omnic);
        router = Router(_router);
        bridgeCanister = _bridgeCanister;
    }

    //----------------------------- brdige canister called  functions ------------------------------

    function handleBridgeMessage(bytes memory _payload)
        external
        onlyBridgeCanister
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
            ) = abi.decode(_payload, (uint8, uint16, uint256, uint256, bytes32));
            router.handleSwap(++nonce, _dstChainId, _dstPoolId, _amountLD, _to);
        }
        return true;
    }

    // called directly by the bridge canister, for IC->EVM swap
    function handleSwap(
        uint16 _dstChainId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        bytes32 _to
    )
        external
        onlyBridgeCanister
        returns (bool)
    {
        router.handleSwap(++nonce, _dstChainId, _dstPoolId, _amountLD, _to);
        return true;
    }

    function revertFailedSwap(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint256 _amountLD,
        bytes32 _to
    ) external onlyBridgeCanister returns (bool) {
        router.revertFailedSwap(_srcChainId, _srcPoolId, _amountLD, _to);
        return true;
    }

    //----------------------------- router called  functions ------------------------------
    function swap(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint16 _dstChainId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        bytes32 _to
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
        _send(OperationTypes.Swap, _payload);
    }

    function addLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint256 _amount
    ) external override onlyRouter {
        bytes memory _payload = abi.encode(
            uint8(OperationTypes.AddLiquidity),
            _srcChainId,
            _srcPoolId,
            _amount
        );
        _send(OperationTypes.AddLiquidity, _payload);
    }

    function removeLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint256 _amount
    ) external override onlyRouter {
        bytes memory _payload = abi.encode(
            uint8(OperationTypes.RemoveLiquidity),
            _srcChainId,
            _srcPoolId,
            _amount
        );
        _send(OperationTypes.RemoveLiquidity, _payload);
    }

    function createPool (
        uint256 _poolId,
        uint8 _sharedDecimals,
        uint8 _localDecimals,
        string memory _name,
        string memory _symbol
    ) external override onlyRouter {
        bytes memory _payload = abi.encode(
            uint8(OperationTypes.CreatePool),
            _poolId,
            _sharedDecimals,
            _localDecimals,
            _name,
            _symbol
        );
        _send(OperationTypes.CreatePool, _payload);
    }

    //--------------------------- set functions------------------------------------------------

    function setBridgeCanister(address _newAddress) public onlyOwner {
        require(_newAddress != address(0x0), "address cannot be 0x0");
        address oldAddress = bridgeCanister;
        bridgeCanister = _newAddress;
        emit UdpateBridgeCanister(oldAddress, _newAddress);
    }

    //----------------------------- internal  functions ------------------------------
    function _send(
        OperationTypes _t,
        bytes memory _payload
    ) internal {
        uint256 _nonce = nonce++;
        omnic.sendMessage(
            chainIdIC,
            TypeCasts.addressToBytes32(bridgeCanister),
            _payload
        );
        emit SendMsg(_t, _nonce);
    }

    function renounceOwnership() public override onlyOwner {}
}
