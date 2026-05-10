// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import { AccessControl } from "@openzeppelin/contracts/access/AccessControl.sol";

/// @title FederationAggregator
/// @notice Skeleton contract for receiving per-firm metric commitments and
///         emitting aggregation events. The full implementation (range proofs,
///         aggregate verification) lands in Phase 2.
/// @dev    In the spike, range-proof verification is replaced by a structural
///         check (`commitment` length == 32). This is sufficient for the
///         end-to-end demo and clearly flagged with `TODO(phase-2)` in the
///         tests.
contract FederationAggregator is AccessControl {
    /// @notice Role permitted to open a new aggregation round.
    bytes32 public constant ROUND_ADMIN_ROLE = keccak256("ROUND_ADMIN_ROLE");

    /// @notice Emitted when a firm submits a contribution.
    event ContributionSubmitted(
        uint256 indexed roundId,
        bytes32 indexed firmIdHash,
        string metric,
        bytes commitment
    );

    /// @notice Emitted when a round is opened for submissions.
    event RoundOpened(uint256 indexed roundId, string metric, uint64 deadline);

    /// @notice Emitted when an aggregate root is finalised for a round.
    event RoundFinalised(uint256 indexed roundId, bytes32 aggregateRoot);

    struct Round {
        string metric;
        uint64 deadline;
        bool finalised;
        bytes32 aggregateRoot;
    }

    /// @notice Round state by `roundId`.
    mapping(uint256 roundId => Round round) public rounds;

    /// @dev Used to enforce one contribution per (round, firm).
    mapping(uint256 roundId => mapping(bytes32 firmIdHash => bool submitted)) private _submitted;

    uint256 public nextRoundId;

    error InvalidCommitment();
    error RoundNotOpen(uint256 roundId);
    error RoundClosed(uint256 roundId);
    error AlreadySubmitted(uint256 roundId, bytes32 firmIdHash);
    error AlreadyFinalised(uint256 roundId);
    error NotRoundAdmin(address caller);

    constructor(address admin) {
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(ROUND_ADMIN_ROLE, admin);
    }

    /// @notice Open a new round for the given metric. Anyone may submit until
    ///         `deadline`.
    function openRound(string calldata metric, uint64 deadline) external returns (uint256 id) {
        if (!hasRole(ROUND_ADMIN_ROLE, msg.sender)) revert NotRoundAdmin(msg.sender);
        id = ++nextRoundId;
        rounds[id] = Round({ metric: metric, deadline: deadline, finalised: false, aggregateRoot: bytes32(0) });
        emit RoundOpened(id, metric, deadline);
    }

    /// @notice Submit a Pedersen commitment to a metric value for a given round.
    /// @param roundId Open round id.
    /// @param firmIdHash SHA-256 hash of the firm identifier.
    /// @param commitment 32-byte compressed Ristretto255 commitment.
    /// @param rangeProof Bulletproofs+ encoding. Ignored in spike phase.
    function submitContribution(
        uint256 roundId,
        bytes32 firmIdHash,
        bytes calldata commitment,
        bytes calldata rangeProof
    ) external {
        Round storage r = rounds[roundId];
        if (r.deadline == 0 || r.finalised) revert RoundNotOpen(roundId);
        if (block.timestamp > r.deadline) revert RoundClosed(roundId);
        if (commitment.length != 32) revert InvalidCommitment();
        if (_submitted[roundId][firmIdHash]) revert AlreadySubmitted(roundId, firmIdHash);
        // TODO(phase-2): verify Bulletproofs+ range proof here.
        // For the spike we accept any non-empty rangeProof or empty.
        rangeProof; // silence compiler unused warning
        _submitted[roundId][firmIdHash] = true;
        emit ContributionSubmitted(roundId, firmIdHash, r.metric, commitment);
    }

    /// @notice Finalise a round by recording its aggregate root. The root is
    ///         computed off-chain by `tyche-fed::aggregate`.
    function finaliseRound(uint256 roundId, bytes32 aggregateRoot) external {
        if (!hasRole(ROUND_ADMIN_ROLE, msg.sender)) revert NotRoundAdmin(msg.sender);
        Round storage r = rounds[roundId];
        if (r.deadline == 0) revert RoundNotOpen(roundId);
        if (r.finalised) revert AlreadyFinalised(roundId);
        r.finalised = true;
        r.aggregateRoot = aggregateRoot;
        emit RoundFinalised(roundId, aggregateRoot);
    }
}
