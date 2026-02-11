#!/bin/bash
# Run the financial extractor with Google Gemini 3 Flash
# Note: Requires gemini_api_key configured in ~/Library/Application Support/dwata/api.toml

NOCODO_LLM_LOG_PAYLOADS=1 \
RUST_LOG=nocodo_llm_sdk::gemini=debug,dwata_agents=info \
cargo run -p dwata-agents --bin financial_extractor -- \
  --eml-path ~/Downloads/Gmail_Receipt_from_Cerebras_Systems_Inc_#2656-4514.eml \
  --provider gemini \
  --model "gemini-3-flash-preview"
