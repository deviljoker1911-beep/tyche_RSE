// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import { AccessControl } from "@openzeppelin/contracts/access/AccessControl.sol";
import { MerkleProof } from "@openzeppelin/contracts/utils/cryptography/MerkleProof.sol";

import { IAttestationRegistry } from "./interfaces/IAttestationRegistry.sol";

/// @title AttestationRegistry
/// @notice Anchors Merkle roots committing to batches of off-chain Tyche
///         attestation records.
/// @dev    The contract is intentionally minimal: it stores only the root,
///         publisher, and timestamp. Records themselves live off-chain;
///         verifiers use this contract together with an inclusion proof
///         (and the leaf bytes) to confirm an attestation was part of an
///         anchored batch.
contract AttestationRegistry is AccessControl, IAttestationRegistry {
    /// @notice Role permitted to publish Merkle roots.
    bytes32 public constant PUBLISHER_ROLE = keccak256("PUBLISHER_ROLE");

    /// @dev Storage layout. Single mapping; no upgradability in the spike.
    struct Record {
        address publisher;
        uint64 timestamp;
        bytes32 batchId;
    }

    mapping(bytes32 root => Record record) private _records;

    /// @notice Thrown when a non-publisher tries to publish.
    error NotPublisher(address caller);
    /// @notice Thrown when the supplied root is zero.
    error ZeroRoot();
    /// @notice Thrown when attempting to publish a root that already exists.
    error AlreadyPublished(bytes32 root);

    constructor(address admin) {
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(PUBLISHER_ROLE, admin);
    }

    /// @inheritdoc IAttestationRegistry
    function publishRoot(bytes32 root, bytes32 batchId) external override {
        if (!hasRole(PUBLISHER_ROLE, msg.sender)) revert NotPublisher(msg.sender);
        if (root == bytes32(0)) revert ZeroRoot();
        if (_records[root].timestamp != 0) revert AlreadyPublished(root);
        _records[root] = Record({
            publisher: msg.sender,
            timestamp: uint64(block.timestamp),
            batchId: batchId
        });
        emit RootPublished(root, batchId, msg.sender, block.timestamp);
    }

    /// @inheritdoc IAttestationRegistry
    function verifyInclusion(
        bytes32 root,
        bytes32 leaf,
        bytes32[] calldata proof
    ) external view override returns (bool ok) {
        if (_records[root].timestamp == 0) return false;
        return MerkleProof.verifyCalldata(proof, root, leaf);
    }

    /// @inheritdoc IAttestationRegistry
    function isPublished(bytes32 root) external view override returns (bool) {
        return _records[root].timestamp != 0;
    }

    /// @inheritdoc IAttestationRegistry
    function publishedAt(bytes32 root) external view override returns (uint256) {
        return _records[root].timestamp;
    }
}
