#!/bin/bash
# Test financial extractor with GPT-5 nano (smallest, fastest model)
# Note: Requires OPENAI_API_KEY environment variable

NOCODO_LLM_LOG_PAYLOADS=1 \
RUST_LOG=nocodo_llm_sdk::openai=debug,dwata_agents=info \
cargo run -p dwata-agents --bin financial_extractor -- \
  --eml-path ~/Downloads/Gmail_Receipt_from_Cerebras_Systems_Inc_#2656-4514.eml \
  --provider openai \
  --model "gpt-5-nano-2025-08-07"
