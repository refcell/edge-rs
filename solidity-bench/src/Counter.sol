// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

contract Counter {
    uint256 public count;

    constructor(uint256 _initial) {
        count = _initial;
    }

    function increment() external {
        count = count + 1;
    }

    function decrement() external {
        count = count - 1;
    }

    function get() external view returns (uint256) {
        return count;
    }

    function reset() external {
        count = 0;
    }
}
