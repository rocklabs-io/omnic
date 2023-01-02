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
    uint256 public erc20FeeBase;
    uint256 public erc20FeeBaseForPerByte;
    bool public feeEnabled;
    bool public erc20Enabled;

    event NativeFeeBase(uint256 nativeFeeBase);
    event NativeFeeForPerByte(uint256 nativeFeeForPerByte);
    event ERC20FeeBase(uint256 erc20FeeBase);
    event ERC20FeeBaseForPerByte(uint256 erc20FeeBaseForPerByte);
    event FeeEnabled(bool feeEnabled);
    event ERC20Enabled(bool erc20Enabled);

    constructor(
        bool _feeEnabled, 
        uint256 _nativeFeeBase, 
        uint256 _nativeFeePerByte
        ) {
        feeEnabled = _feeEnabled;
        nativeFeeBase = _nativeFeeBase;
        nativeFeeForPerByte = _nativeFeePerByte;
    }

    function getFees(bool payInERC20, uint256 msgLength) external view override returns (uint256) {
        if (feeEnabled) {
            if (payInERC20) {
                require(erc20Enabled, "OmnicFeeManager: ERC20 is not enabled");
                return erc20FeeBase.add(erc20FeeBaseForPerByte.mul(msgLength));
            } else {
                return nativeFeeBase.add(nativeFeeForPerByte.mul(msgLength));
            }
        }
        return 0;
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

    function setERC20FeeForPerByte(uint256 _erc20FeeBaseForPerByte) external onlyOwner {
        erc20FeeBaseForPerByte = _erc20FeeBaseForPerByte;
        emit ERC20FeeBase(erc20FeeBaseForPerByte);
    }
}