// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

/// @title IAttestationRegistry
/// @notice External-facing surface for the on-chain Merkle root anchor.
interface IAttestationRegistry {
    /// @notice Emitted when a publisher anchors a Merkle root.
    /// @param root Merkle root (keccak256-based, computed off-chain).
    /// @param batchId Caller-chosen identifier for the batch.
    /// @param publisher Address that called `publishRoot`.
    /// @param timestamp Block timestamp at the time of publication.
    event RootPublished(
        bytes32 indexed root,
        bytes32 indexed batchId,
        address indexed publisher,
        uint256 timestamp
    );

    /// @notice Anchor a Merkle root for a batch of attestations.
    /// @param root Merkle root.
    /// @param batchId Caller-chosen identifier; conventionally the SHA-256 hash
    ///        of the canonical batch metadata.
    function publishRoot(bytes32 root, bytes32 batchId) external;

    /// @notice Verify that a leaf is included under a previously-published root.
    /// @dev The proof format must match OpenZeppelin's MerkleProof library.
    /// @param root A previously-published root.
    /// @param leaf 32-byte leaf hash.
    /// @param proof Sibling hashes from leaf to root.
    /// @return ok True if the proof verifies and `root` was published.
    function verifyInclusion(
        bytes32 root,
        bytes32 leaf,
        bytes32[] calldata proof
    ) external view returns (bool ok);

    /// @notice Whether `root` has been published.
    function isPublished(bytes32 root) external view returns (bool);

    /// @notice Block timestamp at which `root` was published, or zero.
    function publishedAt(bytes32 root) external view returns (uint256);
}
