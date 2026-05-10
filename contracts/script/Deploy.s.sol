// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import { Script } from "forge-std/Script.sol";
import { console2 } from "forge-std/console2.sol";
import { AttestationRegistry } from "../src/AttestationRegistry.sol";
import { FederationAggregator } from "../src/FederationAggregator.sol";

/// @notice Deploys the Tyche contracts to a local Anvil chain.
///
///         Run with `forge script Deploy --rpc-url http://localhost:8545
///         --broadcast --private-key $PRIVATE_KEY`. The first Anvil prefunded
///         account works.
contract Deploy is Script {
    function run() external {
        uint256 pk = vm.envOr("PRIVATE_KEY", uint256(0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80));
        address admin = vm.addr(pk);
        vm.startBroadcast(pk);
        AttestationRegistry registry = new AttestationRegistry(admin);
        FederationAggregator agg = new FederationAggregator(admin);
        vm.stopBroadcast();
        console2.log("AttestationRegistry:", address(registry));
        console2.log("FederationAggregator:", address(agg));
    }
}
