// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import { Test } from "forge-std/Test.sol";
import { AttestationRegistry } from "../src/AttestationRegistry.sol";
import { IAttestationRegistry } from "../src/interfaces/IAttestationRegistry.sol";

contract AttestationRegistryTest is Test {
    AttestationRegistry internal registry;
    address internal admin = address(0xA11CE);
    address internal stranger = address(0xB0B);

    function setUp() public {
        vm.prank(admin);
        registry = new AttestationRegistry(admin);
    }

    function test_admin_can_publish_root() public {
        bytes32 root = keccak256(abi.encodePacked("root-1"));
        bytes32 batch = keccak256(abi.encodePacked("batch-1"));
        vm.prank(admin);
        registry.publishRoot(root, batch);
        assertTrue(registry.isPublished(root));
        assertEq(registry.publishedAt(root), block.timestamp);
    }

    function test_stranger_cannot_publish() public {
        bytes32 root = keccak256(abi.encodePacked("root-2"));
        vm.prank(stranger);
        vm.expectRevert(
            abi.encodeWithSelector(AttestationRegistry.NotPublisher.selector, stranger)
        );
        registry.publishRoot(root, bytes32(0));
    }

    function test_double_publish_reverts() public {
        bytes32 root = keccak256(abi.encodePacked("root-3"));
        vm.prank(admin);
        registry.publishRoot(root, bytes32(0));
        vm.prank(admin);
        vm.expectRevert(abi.encodeWithSelector(AttestationRegistry.AlreadyPublished.selector, root));
        registry.publishRoot(root, bytes32(0));
    }

    function test_zero_root_reverts() public {
        vm.prank(admin);
        vm.expectRevert(AttestationRegistry.ZeroRoot.selector);
        registry.publishRoot(bytes32(0), bytes32(0));
    }

    function test_inclusion_verifies_for_published_root() public {
        // Hand-rolled three-leaf tree using OZ's Merkle conventions:
        //   leaves: [a, b, c]
        //   sorted-pair hash; OZ uses sortedPair semantics in verifyCalldata.
        bytes32 a = keccak256(bytes("a"));
        bytes32 b = keccak256(bytes("b"));
        bytes32 c = keccak256(bytes("c"));
        bytes32 ab = _hashPair(a, b);
        bytes32 cc = _hashPair(c, c);
        bytes32 root = _hashPair(ab, cc);

        vm.prank(admin);
        registry.publishRoot(root, bytes32(uint256(1)));

        bytes32[] memory proof = new bytes32[](2);
        proof[0] = b;
        proof[1] = cc;
        assertTrue(registry.verifyInclusion(root, a, proof));
    }

    function test_inclusion_returns_false_for_unpublished_root() public view {
        bytes32 root = keccak256(abi.encodePacked("nope"));
        bytes32[] memory proof = new bytes32[](0);
        assertFalse(registry.verifyInclusion(root, bytes32(0), proof));
    }

    /// @dev OpenZeppelin MerkleProof uses sorted-pair hashing.
    function _hashPair(bytes32 x, bytes32 y) private pure returns (bytes32) {
        return x < y ? keccak256(abi.encodePacked(x, y)) : keccak256(abi.encodePacked(y, x));
    }
}
