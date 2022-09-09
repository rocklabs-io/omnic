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
    uint256 public nonce;
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

    // todo
    function handle(
        uint16 _srcChainId,
        bytes memory _srcAddress,
        uint64 _nonce,
        bytes memory _payload
    ) external onlyRouter {
        OperationTypes t;
        assembly {
            t := mload(add(_payload, 32))
        }
        if (t == OperationTypes.Swap) {
            router.swap();
        }
    }

    // todo
    function revert() external onlyRouter {}

    function swap(
        uint16 _dstChainId,
        bytes32 _dstRecipientAddress,
        bool _waitOptimistic,
        bytes memory _payload
    ) external onlyRouter {
        _send(_dstChainId, _dstRecipientAddress, _waitOptimistic, _payload);
    }

    function addLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint16 _dstChainId,
        bytes32 _dstRecipientAddress,
        bool _waitOptimistic,
        uint256 _amount
    ) external onlyRouter {
        bytes memory _payload = abi.encode(
            uint8(OperationTypes.AddLiquidity),
            _srcChainId,
            _srcPoolId,
            _amount
        );
        _send(_dstChainId, _dstRecipientAddress, _waitOptimistic, _payload);
    }

    function removeLiquidity(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint16 _dstChainId,
        bytes32 _dstRecipientAddress,
        bool _waitOptimistic,
        uint256 _amount
    ) external onlyRouter {
        bytes memory _payload = abi.encode(
            uint8(OperationTypes.RemoveLiquidity),
            _srcChainId,
            _srcPoolId,
            _amount
        );
        _send(_dstChainId, _dstRecipientAddress, _waitOptimistic, _payload);
    }

    //----------------------------- internal  functions ------------------------------
    function _send(
        uint16 _dstChainId,
        bytes32 _recipientAddress,
        bool _waitOptimistic,
        bytes memory _payload
    ) internal {
        uint256 _nonce = nonce++;
        omnic.sendMessage(
            _dstChainId,
            _recipientAddress,
            _waitOptimistic,
            _payload
        );
        emit SendMsg(_type, _nonce);
    }

    function renounceOwnership() public override onlyOwner {}
}
