# Task: Update Financial Extractor Agent Patterns

## Goal
Update the financial extractor agent so it can reliably produce a valid regex pattern (or multiple patterns) in a single attempt that passes the test suite.

## Scope
- Update the system prompt in `dwata-agents/src/financial_extractor/mod.rs` to reflect the new pattern schema and extraction expectations.
- Update any related types used by the agent output so they align with the new pattern schema (source/destination vendor groups + reference group).
- Optimize the agent behavior so the first response is production-ready and passes the tests without retries.

## Notes
- Test binary: `dwata-agents/src/bin/financial_extractor.rs`
- Helper script to run on a real email: `run_financial_extractor.sh`

## Success Criteria
- The agent produces one or more regex patterns that validate against the new schema and pass the binary test on first attempt.
- Generated patterns include correct group indices for:
  - `amount_group`
  - `source_vendor_group` or `destination_vendor_group`
  - `date_group` (if present)
  - `reference_group` (if present)
