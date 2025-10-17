# Rules
Everything in this crate must be pure, deterministic, atemporal, and side-effect free.
- Deterministic means that given the same inputs, the outputs will always be the same. This is crucial for ensuring that tests can reliably reproduce results and that the system behaves predictably.
- Atemporal means that the system does not rely on or change over time. This ensures that the behavior of the system is consistent regardless of when it is run.
- Side-effect free means that the system does not produce any side effects, such as modifying external state or relying on external systems. This allows for easier testing and debugging, as the system can be treated as a pure function.

# Testing
- New unit tests should be added to the `tests` directory. The tests should cover all new functionality and edge cases.

- Tests should include a plain text description of what the test is checking. This helps in understanding the purpose of the test and makes it easier to maintain.

- Tests should generally have only a few assertions, and only if they are conceptually linked as part of a flow that is being tested (e.g., checking before and after states, the correctness of mutations/reversion). If a test has too many assertions, it may be doing too much and should be split into smaller tests. If a test includes multiple assertions, clearly describe the steps of the flow being checked.

- Note that everything in this crate is deterministic, atemporal, and side-effect free. This means that tests should not rely on any external state or time-based conditions. They should be able to run in any order and produce the same results every time.
    - Tests should not rely on the current time or any external state that could change between runs. This ensures that tests are reliable and can be run in any environment without unexpected failures.
    - Tests should not depend on the passage of time. In particular, methods like `observe_rtt_ms_to_host(t)` accept input directly as a parameter, rather than internally reading the current time.

- When writing tests, consider edge cases and boundary conditions. This helps to ensure that the system behaves correctly in all scenarios.


## Testing *user-written* implementation code
- When writing tests, on code that has been written before the agent's current task, the agent must first analyze the existing code within the context of the overall codebase, along with any comments or descriptions that explain its purpose and functionality.
- The users code may contain errors or inconsistencies! The agents task is to be BRUTALLY CRITICAL of existing code, and to attempt to write tests that will poke holes in the code's correctness and reveal any bugs or issues.
- The agent should identify and address these issues in the tests, ensuring that the tests accurately reflect the intended behavior of the code. It's ok if the tests reveal bugs or issues in the existing code; the goal is to ensure that the code behaves correctly and reliably. Failing tests are ok, and should be reported back to the user for fixing.
- If the agent identifies any issues or inconsistencies in the existing code while writing tests, it should document these findings and provide suggestions for how to address them. However, the agent **must not** modify the existing code unless explicitly instructed to do so by the user.

## Testing *agent-written* implementation code
- When writing tests on code that the agent has written itself during the current task, the agent should ensure that the tests are comprehensive and cover all relevant scenarios for the newly written code.

- The agent should write tests that cover the expected behavior of the newly written code, including edge cases and potential failure modes.

- When writing tests for its own code, the agen may iteratively refine both the code and the tests to ensure that they are correct and reliable.


## Multiple cases in one test
When it makes sense to repeatedly test a single function on multiple *input* cases, use a `#[test_case(data; "data case description")]` attribute on a test to specify the data cases. This allows the test to be run multiple times with different inputs, and will report each case separately in the test results.

This is "DRY"er than writing a separate test function for each case, and cleaner than putting multiple assertion statements in a single test function that loops over the data cases.

For example:
```rust
#[test_case(0 ; "0u64")]
#[test_case(1 ; "1u64")]
#[test_case(u32::MAX as u64 ; "u32::MAX as u64")]
#[test_case(u64::MAX ; "u64::MAX")]
#[test_case(u64::MAX - 1 ; "u64::MAX - 1")]
#[test_case(0x1234_5678_9abc_def0 ; "0x1234_5678_9abc_def0")]
fn test_split_u64_roundtrip(val: u64) {
    let parts = split_u64(val);
    assert_eq!(join_u32s(parts[0], parts[1]), val);
}
```
