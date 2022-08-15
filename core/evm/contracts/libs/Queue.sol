// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.9;

import {Types} from "./Types.sol";

/**
 * @title QueueLib
 * @author Illusory Systems Inc.
 * @notice Library containing queue struct and operations for queue used by
 * Home and Replica.
 **/
library QueueLib {
    using Types for Types.MessageFormat;
    
    /**
     * @notice Queue struct
     * @dev Internally keeps track of the `first` and `last` elements through
     * indices and a mapping of indices to enqueued elements.
     **/
    struct Queue {
        uint128 first;
        uint128 last;
        mapping(uint256 => Types.MessageFormat) queue;
    }

    /**
     * @notice Initializes the queue
     * @dev Empty state denoted by _q.first > q._last. Queue initialized
     * with _q.first = 1 and _q.last = 0.
     **/
    function initialize(Queue storage _q) internal {
        if (_q.first == 0) {
            _q.first = 1;
        }
    }

    /**
     * @notice Enqueues a single new element
     * @param _item New element to be enqueued
     * @return _last Index of newly enqueued element
     **/
    function enqueue(Queue storage _q, Types.MessageFormat memory _item)
        internal
        returns (uint128 _last)
    {
        _last = _q.last + 1;
        _q.last = _last;
        if (_item.payload.length != 0){
            _q.queue[_last] = _item;
        }
    }

    /**
     * @notice Dequeues element at front of queue
     * @dev Removes dequeued element from storage
     * @return _item Dequeued element
     **/
    function dequeue(Queue storage _q) internal returns (Types.MessageFormat memory _item) {
        uint128 _last = _q.last;
        uint128 _first = _q.first;
        require(_length(_last, _first) != 0, "Empty");
        _item = _q.queue[_first];
        if (_item.payload.length != 0) {
            // saves gas if we're dequeuing 0
            delete _q.queue[_first];
        }

        _q.first = _first + 1;
    }

    /**
     * @notice Batch enqueues several elements
     * @param _items Array of elements to be enqueued
     * @return _last Index of last enqueued element
     **/
    function enqueue(Queue storage _q, Types.MessageFormat[] memory _items)
        internal
        returns (uint128 _last)
    {
        _last = _q.last;
        for (uint256 i = 0; i < _items.length; i += 1) {
            _last += 1;
            Types.MessageFormat memory _item = _items[i];
            if (_item.payload.length != 0) {
                _q.queue[_last] = _item;
            }
        }
        _q.last = _last;
    }

    /**
     * @notice Batch dequeues `_number` elements
     * @dev Reverts if `_number` > queue length
     * @param _number Number of elements to dequeue
     * @return Array of dequeued elements
     **/
    function dequeue(Queue storage _q, uint256 _number)
        internal
        returns (Types.MessageFormat[] memory)
    {
        uint128 _last = _q.last;
        uint128 _first = _q.first;
        // Cannot underflow unless state is corrupted
        require(_length(_last, _first) >= _number, "Insufficient");

        Types.MessageFormat[] memory _items = new Types.MessageFormat[](_number);

        for (uint256 i = 0; i < _number; i++) {
            _items[i] = _q.queue[_first];
            delete _q.queue[_first];
            _first++;
        }
        _q.first = _first;
        return _items;
    }

    /**
     * @notice Returns true if `_item` is in the queue and false if otherwise
     * @dev Linearly scans from _q.first to _q.last looking for `_item`
     * @param _item Item being searched for in queue
     * @return True if `_item` currently exists in queue, false if otherwise
     **/
    function contains(Queue storage _q, Types.MessageFormat memory _item)
        internal
        view
        returns (bool)
    {
        for (uint256 i = _q.first; i <= _q.last; i++) {
            if (keccak256(_q.queue[i].payload) == keccak256(_item.payload)) {
                return true;
            }
        }
        return false;
    }

    /// @notice Returns last item in queue
    /// @dev Returns bytes32(0) if queue empty
    function lastItem(Queue storage _q) internal view returns (Types.MessageFormat memory) {
        return _q.queue[_q.last];
    }

    /// @notice Returns element at front of queue without removing element
    /// @dev Reverts if queue is empty
    function peek(Queue storage _q) internal view returns (Types.MessageFormat memory _item) {
        require(!isEmpty(_q), "Empty");
        _item = _q.queue[_q.first];
    }

    /// @notice Returns true if queue is empty and false if otherwise
    function isEmpty(Queue storage _q) internal view returns (bool) {
        return _q.last < _q.first;
    }

    /// @notice Returns number of elements in queue
    function length(Queue storage _q) internal view returns (uint256) {
        uint128 _last = _q.last;
        uint128 _first = _q.first;
        // Cannot underflow unless state is corrupted
        return _length(_last, _first);
    }

    /// @notice Returns number of elements between `_last` and `_first` (used internally)
    function _length(uint128 _last, uint128 _first)
        internal
        pure
        returns (uint256)
    {
        return uint256(_last + 1 - _first);
    }
}
