// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.9;
pragma abicoder v2;

// imports
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";

import "./FactoryPool.sol";
import "./Pool.sol";
import "./Bridge.sol";
import "./interfaces/IBridgeRouter.sol";
import {TypeCasts} from "./utils/TypeCasts.sol";

contract Router is IBridgeRouter, Ownable, ReentrancyGuard {
    using SafeMath for uint256;

    //------------------------------- variables --------------------------------------------
    uint16 public chainId;
    FactoryPool public factory; // used for creating pools
    Bridge public localBridge;

    //----------------------------- events ----------------------------------------------

    event HandleSwap(
        uint256 nonce,
        uint16 dstChainId,
        uint256 dstPoolId,
        uint256 amount,
        bytes32 to
    );
    event Revert(
        uint16 srcChainId,
        uint256 _srcPoolId,
        uint256 _amount,
        bytes32 _to
    );

    //---------------------------------------------------------------------------

    modifier onlyLocalBridge() {
        require(
            msg.sender == address(localBridge),
            "Bridge: caller must be Bridge."
        );
        _;
    }

    constructor() {
        chainId = uint16(block.chainid);
    }

    // must be called after deployment
    function setBridgeAndFactory(address _bridge, address _factory)
        external
        onlyOwner
    {
        // require(
        //     address(localBridge) == address(0x0) &&
        //         address(factory) == address(0x0),
        //     "bridge and factory already initialized"
        // ); // 1 time only
        require(_bridge != address(0x0), "bridge cant be 0x0");
        require(_factory != address(0x0), "factory cant be 0x0");

        localBridge = Bridge(_bridge);
        factory = FactoryPool(_factory);
    }

    //--------------------------- main functions------------------------------------------------
    function addLiquidity(
        uint256 _poolId,
        uint256 _amountLD,
        address _to
    ) external override nonReentrant {
        Pool pool = _getPool(_poolId);
        _safeTransferFrom(pool.token(), msg.sender, address(pool), _amountLD);
        pool.addLiquidity(_to, _amountLD);
        // send message to bridge on ic
        localBridge.addLiquidity(chainId, _poolId, _amountLD);
    }

    function removeLiquidity(
        uint16 _srcPoolId,
        uint256 _amountLP,
        address _to
    ) external override nonReentrant {
        require(_amountLP > 0, "insufficient lp");
        Pool pool = _getPool(_srcPoolId);
        uint256 amountLD = pool.removeLiquidity(msg.sender, _amountLP, _to);

        // send message to bridge on ic
        localBridge.removeLiquidity(chainId, _srcPoolId, amountLD);
    }

    function swap(
        uint16 _dstChainId,
        uint256 _srcPoolId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        uint256 _minAmountLD,
        bytes32 _to
    ) external override nonReentrant {
        require(_amountLD > 0, "cannot swap 0");
        Pool pool = _getPool(_srcPoolId);
        {
            uint256 convertRate = pool.convertRate();
            _amountLD = _amountLD.div(convertRate).mul(convertRate);
        }
        _safeTransferFrom(pool.token(), msg.sender, address(pool), _amountLD);
        //event
        pool.swap(_dstChainId, _dstPoolId, msg.sender, _amountLD, _minAmountLD);

        localBridge.swap(
            chainId,
            _srcPoolId,
            _dstChainId,
            _dstPoolId,
            _amountLD,
            _to
        );
    }

    function handleSwap(
        uint256 _nonce,
        uint16 _dstChainId,
        uint256 _dstPoolId,
        uint256 _amountLD,
        bytes32 _to
    ) external override nonReentrant onlyLocalBridge {
        require(
            _dstChainId == chainId,
            "destination chain id is not this chain."
        );
        require(_amountLD > 0, "cannot swap 0");
        Pool pool = _getPool(_dstPoolId);
        {
            uint256 convertRate = pool.convertRate();
            _amountLD = _amountLD.div(convertRate).mul(convertRate);
        }
        _safeTransferFrom(
            pool.token(),
            address(pool),
            TypeCasts.bytes32ToAddress(_to),
            _amountLD
        );
        //event
        emit HandleSwap(_nonce, _dstChainId, _dstPoolId, _amountLD, _to);
    }

    function revertFailedSwap(
        uint16 _srcChainId,
        uint256 _srcPoolId,
        uint256 _amountLD,
        bytes32 _to
    ) external override nonReentrant onlyLocalBridge {
        require(
            _srcChainId == chainId,
            "destination chain id is not this chain."
        );
        require(_amountLD > 0, "cannot swap 0");
        Pool pool = _getPool(_srcPoolId);
        {
            uint256 convertRate = pool.convertRate();
            _amountLD = _amountLD.div(convertRate).mul(convertRate);
        }
        _safeTransferFrom(
            pool.token(),
            address(pool),
            TypeCasts.bytes32ToAddress(_to),
            _amountLD
        );
        //event
        emit Revert(_srcChainId, _srcPoolId, _amountLD, _to);
    }

    //--------------------------- config functions------------------------------------------------

    function createPool(
        address _token,
        uint8 _sharedDecimals,
        uint8 _localDecimals,
        string memory _name,
        string memory _symbol
    ) external onlyOwner returns (address) {
        require(_token != address(0x0), "_token cannot be 0x0");
        
        (address pool, uint256 poolId) = factory.createPool(
                _token,
                _sharedDecimals,
                _localDecimals,
                _name,
                _symbol
            );
        // send message to bridge on ic to create according pool automatically
        localBridge.createPool(poolId, _sharedDecimals, _localDecimals, _name, _symbol);
        return pool;
    }

    //----------------------------- internal  functions ------------------------------

    function _getPool(uint256 _poolId) internal view returns (Pool pool) {
        pool = factory.pools(_poolId);
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
