# Testing
- New unit tests should be added to the `tests` directory. The tests should cover all new functionality and edge cases.

- Tests should include a plain text description of what the test is checking. This helps in understanding the purpose of the test and makes it easier to maintain.

- Tests should generally have only a few assertions, and only if they are conceptually linked as part of a flow that is being tested (e.g., checking before and after states, the correctness of mutations/reversion). If a test has too many assertions, it may be doing too much and should be split into smaller tests. If a test includes multiple assertions, clearly describe the steps of the flow being checked.