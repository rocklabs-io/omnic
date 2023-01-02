// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.9;

// external
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/math/SafeMath.sol";

// internal
import "./interfaces/IOmnicFeeManager.sol";

contract OmnicFeeManager is IOmnicFeeManager, Ownable {
    using SafeMath for uint256;

    uint256 public nativeFeeBase;
    uint256 public nativeFeeForPerByte;

    address public erc20FeeToken;
    uint256 public erc20FeeBase;
    uint256 public erc20FeeForPerByte;

    bool public feeEnabled;
    bool public erc20Enabled;

    event NativeFeeBase(uint256 nativeFeeBase);
    event NativeFeeForPerByte(uint256 nativeFeeForPerByte);
    event ERC20FeeBase(uint256 erc20FeeBase);
    event ERC20FeeForPerByte(uint256 erc20FeeForPerByte);
    event FeeEnabled(bool feeEnabled);
    event ERC20Enabled(bool erc20Enabled);

    constructor(
        bool _feeEnabled, 
        bool _erc20Enabled,
        address _erc20FeeToken,
        uint256 _nativeFeeBase, 
        uint256 _nativeFeePerByte
        ) {
        feeEnabled = _feeEnabled;
        erc20Enabled = _erc20Enabled;
        erc20FeeToken = _erc20FeeToken;

        nativeFeeBase = _nativeFeeBase;
        nativeFeeForPerByte = _nativeFeePerByte;
        erc20FeeBase = 0;
        erc20FeeForPerByte = 0;
    }

    function getFees(bool payInERC20, uint256 msgLength) external view override returns (uint256) {
        if (feeEnabled) {
            if (payInERC20) {
                return erc20FeeBase.add(erc20FeeForPerByte.mul(msgLength));
            } else {
                return nativeFeeBase.add(nativeFeeForPerByte.mul(msgLength));
            }
        }
        return 0;
    }

    function getERC20FeeToken() external view override returns (address) {
        return erc20FeeToken;
    }

    function setFeeEnabled(bool _feeEnabled) external onlyOwner {
        feeEnabled = _feeEnabled;
        emit FeeEnabled(feeEnabled);
    }

    function setERC20Enabled(bool _erc20Enabled) external onlyOwner {
        erc20Enabled = _erc20Enabled;
        emit ERC20Enabled(erc20Enabled);
    }

    function setNativeFeeBase(uint256 _nativeFeeBase) external onlyOwner {
        nativeFeeBase = _nativeFeeBase;
        emit NativeFeeBase(nativeFeeBase);
    }

    function setERC20FeeBase(uint256 _erc20FeeBase) external onlyOwner {
        erc20FeeBase = _erc20FeeBase;
        emit ERC20FeeBase(erc20FeeBase);
    }

    function setNativeFeeForPerByte(uint256 _nativeFeeForPerByte) external onlyOwner {
        nativeFeeForPerByte = _nativeFeeForPerByte;
        emit NativeFeeBase(nativeFeeForPerByte);
    }

    function setERC20FeeForPerByte(uint256 _erc20FeeForPerByte) external onlyOwner {
        erc20FeeForPerByte = _erc20FeeForPerByte;
        emit ERC20FeeForPerByte(erc20FeeForPerByte);
    }
}