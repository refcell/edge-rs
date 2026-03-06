// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "../src/ERC20.sol";

/// @dev Gas benchmarks for fair comparison with Edge.
/// setUp mints tokens so storage is pre-populated. Each test function does one
/// measured call. Storage slots are cold at the start of each test (forge
/// reverts to snapshot between tests), but have non-zero original values from setUp.
contract ERC20GasTest is Test {
    ERC20 token;
    address deployer;
    address alice;
    address bob;
    address spender;

    function setUp() public {
        deployer = address(this);
        alice = makeAddr("alice");
        bob = makeAddr("bob");
        spender = makeAddr("spender");

        token = new ERC20();
        // Seed state to match Edge benchmark
        token.mint(deployer, 100_000);
        token.mint(alice, 50_000);
        token.transfer(bob, 1); // give bob nonzero balance
        token.approve(spender, 50_000);
    }

    function test_gas_totalSupply() public view {
        token.totalSupply();
    }

    function test_gas_balanceOf() public view {
        token.balanceOf(alice);
    }

    function test_gas_transfer() public {
        token.transfer(bob, 100);
    }

    function test_gas_approve() public {
        token.approve(spender, 10_000);
    }

    function test_gas_allowance() public view {
        token.allowance(deployer, spender);
    }

    function test_gas_transferFrom() public {
        vm.prank(spender);
        token.transferFrom(deployer, alice, 100);
    }

    function test_gas_mint() public {
        token.mint(alice, 100);
    }

    function test_gas_deploy() public {
        new ERC20();
    }
}
