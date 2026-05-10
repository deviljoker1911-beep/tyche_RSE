// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { FederationAggregator } from "../src/FederationAggregator.sol";

contract FederationAggregatorTest is Test {
    FederationAggregator internal agg;
    address internal admin = address(0xA11CE);
    bytes32 internal firmA = keccak256("firm-a");
    bytes32 internal firmB = keccak256("firm-b");

    function setUp() public {
        vm.prank(admin);
        agg = new FederationAggregator(admin);
    }

    function test_open_round_and_submit() public {
        vm.prank(admin);
        uint256 id = agg.openRound("expected_loss", uint64(block.timestamp + 1 days));

        bytes memory commitment = new bytes(32);
        // Fill with arbitrary non-zero bytes; the spike contract only checks length.
        for (uint256 i = 0; i < 32; i++) commitment[i] = bytes1(uint8(i + 1));

        agg.submitContribution(id, firmA, commitment, "");
        agg.submitContribution(id, firmB, commitment, "");
    }

    function test_double_submit_reverts() public {
        vm.prank(admin);
        uint256 id = agg.openRound("var_99", uint64(block.timestamp + 1 days));
        bytes memory commitment = new bytes(32);
        agg.submitContribution(id, firmA, commitment, "");
        vm.expectRevert(
            abi.encodeWithSelector(FederationAggregator.AlreadySubmitted.selector, id, firmA)
        );
        agg.submitContribution(id, firmA, commitment, "");
    }

    function test_invalid_commitment_length_reverts() public {
        vm.prank(admin);
        uint256 id = agg.openRound("var_99", uint64(block.timestamp + 1 days));
        bytes memory bad = new bytes(31);
        vm.expectRevert(FederationAggregator.InvalidCommitment.selector);
        agg.submitContribution(id, firmA, bad, "");
    }

    function test_after_deadline_reverts() public {
        vm.prank(admin);
        uint256 id = agg.openRound("x", uint64(block.timestamp + 1));
        skip(2);
        bytes memory commitment = new bytes(32);
        vm.expectRevert(abi.encodeWithSelector(FederationAggregator.RoundClosed.selector, id));
        agg.submitContribution(id, firmA, commitment, "");
    }

    function test_finalise_round() public {
        vm.prank(admin);
        uint256 id = agg.openRound("x", uint64(block.timestamp + 1 days));
        bytes32 root = keccak256("agg-root");
        vm.prank(admin);
        agg.finaliseRound(id, root);
        (string memory metric, uint64 deadline, bool finalised, bytes32 storedRoot) =
            agg.rounds(id);
        metric;
        deadline;
        assertTrue(finalised);
        assertEq(storedRoot, root);
    }
}
