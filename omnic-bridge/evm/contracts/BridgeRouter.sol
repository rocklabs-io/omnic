// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;
pragma abicoder v2;

// imports
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/math/SafeMath.sol";

import "./FactoryPool.sol";
import "./Pool.sol";
import "./Bridge.sol";
import "./interfaces/IBridgeRouter.sol";
import {addressToBytes32} from "./utils/TypeCasts.sol";

contract Router is IBridgeRouter, Ownable, ReentrancyGuard {
    using SafeMath for uint256;

    //------------------------------- variables --------------------------------------------
    uint256 public chainId;
    Factory public factory; // used for creating pools
    Bridge public bridge;
    uint16 public chainIdOnIC;
    address public relayerBridgeOnIC;

    //----------------------------- events ----------------------------------------------

    event UdpateRelayerBridgeOnIC(address oldAddrss, addresss newAddrss);
    event Revert(
        uint8 bridgeFunctionType,
        uint16 chainId,
        bytes srcAddress,
        uint256 nonce
    );

    //---------------------------------------------------------------------------

    modifier onlyBridgeOnIC() {
        require(
            msg.sender == address(bridgeOnIC),
            "Bridge: caller must be Bridge."
        );
        _;
    }

    constructor(uint16 _chainIdOnIC, address _relayerBridgeOnIC) {
        require(_relayerBridgeOnIC != address(0x0), "address cannot be 0x0");
        uint256 _chainId;
        assembly {
            _chainId := chainid()
        }
        chainId = _chianId;
        chainIdOnIC = _chainIdOnIC;
        relayerBridgeOnIC = _relayerBridgeOnIC;
    }

    function setBridgeAndFactory(Bridge _bridge, Factory _factory)
        external
        onlyOwner
    {
        require(
            address(bridge) == address(0x0) && address(factory) == address(0x0),
            "bridge and factory already initialized"
        ); // 1 time only
        require(address(_bridge) != address(0x0), "bridge cant be 0x0");
        require(address(_factory) != address(0x0), "factory cant be 0x0");

        bridge = _bridge;
        factory = _factory;
    }

    //--------------------------- main functions------------------------------------------------
    function addLiquidity(
        uint256 _poolId,
        uint256 _amountLD,
        address _to
    ) external override nonReentrant {
        Pool pool = _getPool(_poolId);
        _safeTransferFrom(pool.token(), msg.sender, address(pool), _amountLD);
        pool.mint(_to, _amountLD);
        // send message to bridge on ic
        bridge.addLiquidity(
            chainId,
            _poolId,
            chainIdOnIC,
            addressToBytes32(relayerBridgeOnIC),
            true,
            _amountLD
        );
    }

    function removeLiquidity(
        uint16 _srcPoolId,
        uint256 _amountLP,
        address _to
    ) external override nonReentrant {
        require(_amountLP > 0, "insufficient lp");
        Pool pool = _getPool(_srcPoolId);
        amountLD = pool.removeLiquidity(msg.sender, _amountLP, _to);

        // send message to bridge on ic
        bridge.removeLiquidity(
            chainId,
            _srcPoolId,
            chainIdOnIC,
            addressToBytes32(relayerBridgeOnIC),
            true,
            amountLD
        );
    }

    function swap(
        uint16 _dstChainId,
        uint256 _srcPoolId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        uint256 _minAmountLD,
        bytes32 _to
    ) external payable override nonReentrant {
        require(_amountLD > 0, "Stargate: cannot swap 0");
        Pool pool = _getPool(_srcPoolId);
        {
            uint256 convertRate = pool.convertRate();
            _amountLD = _amountLD.div(convertRate).mul(convertRate);
        }
        _safeTransferFrom(pool.token(), msg.sender, address(pool), _amountLD);
        //event
        pool.swap(
            _dstChainId,
            _dstPoolId,
            msg.sender,
            _amountLD,
            _minAmountLD,
            true
        );

        bytes memory _payload = abi.encode(
            uint8(bridge.OperationTypes.Swap),
            chainId,
            _srcPoolId,
            _dstChainId,
            _dstPoolId,
            _amountLD,
            _to
        );

        bridge.swap(
            hainId,
            _srcPoolId,
            chainIdOnIC,
            addressToBytes32(relayerBridgeOnIC),
            _payload
        );
    }

    //--------------------------- config functions------------------------------------------------

    function setRelayerBridgeOnIC(address _newAddress) public onlyOwner {
        require(_newAddress != address(0x0), "address cannot be 0x0");
        address oldAddress = relayerBridgeOnIC;
        relayerBridgeOnIC = _newAddress;
        emit UdpateRelayerBridgeOnIC(oldAddrss, _newAddrss);
    }

    function createPool(
        uint256 _poolId,
        address _token,
        uint8 _sharedDecimals,
        uint8 _localDecimals,
        string memory _name,
        string memory _symbol
    ) external onlyOwner returns (address) {
        require(_token != address(0x0), "_token cannot be 0x0");
        return
            factory.createPool(
                _poolId,
                _token,
                _sharedDecimals,
                _localDecimals,
                _name,
                _symbol
            );
    }

    //----------------------------- internal  functions ------------------------------

    function _getPool(uint256 _poolId) internal view returns (Pool pool) {
        pool = factory.getPool(_poolId);
        require(address(pool) != address(0x0), "Pool does not exist");
    }

    function _safeTransferFrom(
        address token,
        address from,
        address to,
        uint256 value
    ) private {
        bytes4 selector = bytes4(
            keccak256(bytes("transferFrom(address,address,uint256)"))
        );
        (bool success, bytes memory data) = token.call(
            abi.encodeWithSelector(selector, from, to, value)
        );
        require(
            success && (data.length == 0 || abi.decode(data, (bool))),
            "TRANSFER_FROM_FAILED"
        );
    }
}
