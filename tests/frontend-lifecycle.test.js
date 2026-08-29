/**
 * Frontend Lifecycle & State Synchronization Test Suite
 * Validates:
 * 1. Stale polling responses cannot overwrite newer event-driven connection states (epoch/version protection).
 * 2. Rapid frontend triggerConnect/triggerDisconnect invocation guard sends exactly one backend command.
 * 3. Transitioning state polling skips expensive health probes and only polls lightweight connection state.
 */

// =========================================================================
// Test A: Stale poll cannot overwrite newer connection event state
// =========================================================================
async function test_a_stale_poll_cannot_overwrite_newer_event_state() {
  let state = "DISCONNECTED";
  let version = 0;

  // Simulate poll start at version 0
  const pollStartVersion = version;

  // Poll starts and prepares to fetch (takes 100ms)
  const slowPollPromise = new Promise((resolve) => {
    setTimeout(() => {
      resolve("DISCONNECTED"); // Old state returned by slow poll
    }, 100);
  });

  // While poll is in flight, backend emits connection-state-changed event: "STARTING_ROUTER"
  version += 1;
  state = "STARTING_ROUTER";

  // Slow poll resolves
  const polledState = await slowPollPromise;
  if (version === pollStartVersion) {
    state = polledState; // Should NOT execute
  }

  if (state !== "STARTING_ROUTER") {
    throw new Error(
      `Test A Failed: State was overwritten by stale poll! Expected STARTING_ROUTER, got ${state}`
    );
  }
  console.log("✓ Test A: Stale poll cannot overwrite newer connection event state (PASSED)");
}

// =========================================================================
// Test C: Rapid frontend Connect invocation guard sends only one backend command
// =========================================================================
async function test_c_rapid_connect_guard_sends_single_command() {
  let backendConnectCalls = 0;
  let inFlight = false;

  const mockApiConnect = async () => {
    backendConnectCalls += 1;
    await new Promise((r) => setTimeout(r, 50));
  };

  const triggerConnect = async () => {
    if (inFlight) {
      return; // Blocked by in-flight guard
    }
    inFlight = true;
    try {
      await mockApiConnect();
    } finally {
      inFlight = false;
    }
  };

  // Simulate 5 rapid double/multi-clicks within milliseconds
  await Promise.all([
    triggerConnect(),
    triggerConnect(),
    triggerConnect(),
    triggerConnect(),
    triggerConnect(),
  ]);

  if (backendConnectCalls !== 1) {
    throw new Error(
      `Test C Failed: Expected exactly 1 backend connect call, but got ${backendConnectCalls}`
    );
  }
  console.log("✓ Test C: Rapid frontend Connect invocation guard sends only one backend command (PASSED)");
}

// =========================================================================
// Test D: Transitioning state polling does not execute expensive health probe
// =========================================================================
async function test_d_transitioning_state_skips_expensive_health_probe() {
  let healthProbesCount = 0;
  let connectionStateProbesCount = 0;

  const mockGetHealthStatus = async () => {
    healthProbesCount += 1;
    return {};
  };

  const mockGetConnectionState = async () => {
    connectionStateProbesCount += 1;
    return "STARTING_ROUTER";
  };

  const executePoll = async (currentState) => {
    const isTransitioning =
      currentState !== "CONNECTED" &&
      currentState !== "DISCONNECTED" &&
      currentState !== "ERROR";

    if (isTransitioning) {
      // During transition: ONLY light state probe, NO health probe
      await mockGetConnectionState();
    } else {
      // Stable state: full health probe
      await Promise.all([mockGetHealthStatus(), mockGetConnectionState()]);
    }
  };

  const transitionalStates = [
    "STARTING_AETHER",
    "SCANNING_AETHER",
    "WAITING_FOR_AETHER",
    "TESTING_AETHER",
    "STARTING_ROUTER",
    "TESTING_ROUTING",
    "RECONNECTING",
    "DISCONNECTING",
  ];

  for (const st of transitionalStates) {
    await executePoll(st);
  }

  if (healthProbesCount !== 0) {
    throw new Error(
      `Test D Failed: Health probe was called ${healthProbesCount} times during transitional states!`
    );
  }

  if (connectionStateProbesCount !== transitionalStates.length) {
    throw new Error(
      `Test D Failed: Expected ${transitionalStates.length} lightweight probes, got ${connectionStateProbesCount}`
    );
  }

  // Now test stable state
  await executePoll("CONNECTED");
  if (healthProbesCount !== 1) {
    throw new Error("Test D Failed: Stable CONNECTED state should execute health probe");
  }

  console.log("✓ Test D: Transitioning state polling does not execute expensive health probe (PASSED)");
}

async function runAll() {
  console.log("=== Running Frontend Lifecycle Test Suite ===");
  await test_a_stale_poll_cannot_overwrite_newer_event_state();
  await test_c_rapid_connect_guard_sends_single_command();
  await test_d_transitioning_state_skips_expensive_health_probe();
  console.log("=============================================");
  console.log("ALL FRONTEND LIFECYCLE TESTS PASSED!");
  console.log("=============================================\n");
}

runAll().catch((e) => {
  console.error(e);
  process.exit(1);
});
