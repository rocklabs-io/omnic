pragma solidity ^0.8.9;

import {Omnic} from "./Omnic";
import "@openzeppelin/contracts/access/Ownable.sol";


contract XAppConnectionManager is Ownable {

    // Omnic contract
    Omnic public omnic;

    // ChainId => Omnic contract Address
    mapping(uint32 => address) public chainIdToOmnic;
    // Omnic contract Address => chainId
    mapping(address => uint32) public omnicToChainId;

    constructor() Ownable() {}


    event OmnicEnrolled(uint32 indexed chainId, address omnic);
    event OmnicUnenrolled(uint32 indexed chainId, address omnic);


    function setOmnic(address _omnic) external onlyOwner {
        omnic = Omnic(_omnic);
    }


    function enrollOmnic(address _omnic, uint32 _chainId) external onlyOwner {
        // unenroll any existing omnic first
        _unenrollOmnic(_omnic);
        chainIdToOmnic[_chainId] = _omnic;
        omnicToChainId[_omnic] = _chainId;
        emit OmnicEnrolled(_chainId, _omnic);
    }

    function unenrollOmnic(address _omnic) external onlyOwner {
        _unenrollOmnic(_omnic);
    }

    function _unenrollOmnic(address _omnic) internal {
        uint32 _currentChainId = omnicToChainId[_omnic];
        chainIdToOmnic[_currentChainId] = address(0);
        omnicToChainId[_omnic] = 0;
        emit OmnicUnenrolled(_currentChainId, _omnic);
    }

    function isOmnicContract(address _omnic) external view returns (bool) {
        return omnicToChainId(_omnic) != 0;
    }

    function localChainId() external view returns (uint32) {
        return omnic.chainId();
    }

}