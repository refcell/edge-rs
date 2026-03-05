// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "../src/Counter.sol";

/// @dev Gas benchmarks for fair comparison with Edge.
/// Each test deploys fresh so storage is cold on first access.
/// "Cold" tests: deploy with initial=0, single call (first ever access to slot).
/// "Warm original" tests: deploy with initial=5 (slot already non-zero), single call.
/// This matches Edge's revm setup where each call() is a new transaction.
contract CounterColdTest is Test {
    /// Deploy + single cold get() (SLOAD on slot with original=0)
    function test_gas_get_cold_zero() public {
        Counter c = new Counter(0);
        c.get();
    }

    /// Deploy with initial=5, then get() (SLOAD on slot with original=5, cold in this tx)
    function test_gas_get_cold_nonzero() public {
        Counter c = new Counter(5);
        c.get();
    }

    /// Deploy with initial=0, then increment (0→1, cold SSTORE zero→nonzero)
    function test_gas_increment_cold_from_zero() public {
        Counter c = new Counter(0);
        c.increment();
    }

    /// Deploy with initial=5, then increment (5→6, cold SSTORE nonzero→nonzero)
    function test_gas_increment_cold_from_nonzero() public {
        Counter c = new Counter(5);
        c.increment();
    }

    /// Deploy with initial=5, then decrement (5→4, cold SSTORE nonzero→nonzero)
    function test_gas_decrement_cold_nonzero() public {
        Counter c = new Counter(5);
        c.decrement();
    }

    /// Deploy with initial=5, then reset (5→0, cold SSTORE nonzero→zero, gets refund)
    function test_gas_reset_cold_nonzero() public {
        Counter c = new Counter(5);
        c.reset();
    }

    /// Just deploy
    function test_gas_deploy_zero() public {
        new Counter(0);
    }

    function test_gas_deploy_nonzero() public {
        new Counter(5);
    }
}
