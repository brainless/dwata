# Extracting Financial Data from Emails Without Reading Every Email with an LLM

> **This is a work in progress.** The pipeline described here is functional and testable today, but several pieces — user review queues, vendor resolution, and full type-safe output — are still being built. We are publishing this early because the core architecture is settled and we think the approach is worth sharing.

---

Your inbox is a ledger. Every utility bill, bank debit alert, SaaS subscription renewal, and merchant receipt is a financial event — timestamped, signed by a real sender, and carrying structured data inside what looks like unstructured prose.

The question is how to get that data out reliably, cheaply, and without handing every email to a large language model.

This post explains how Dwata approaches the problem.

---

## The Problem Space

Financial emails from a given sender are not random. An electricity bill from your utility provider this month looks almost identical to last month's bill, and the month before that. The subject lines, layout, and phrasing are generated from a template; only a handful of values change: amount, billing period, due date, account number.

This repeatability is a structural property you can exploit. The goal is to discover that template automatically, name its variables semantically, and then use it to parse every email from that sender — without calling an LLM for each one.

---

## How Other Tools Solve This

Before explaining our approach, it is worth surveying what else exists.

### Bank APIs (Plaid, Open Banking / PSD2)

The cleanest source of financial data is the bank itself. Tools like [Plaid](https://plaid.com/) and the European Open Banking standard let applications fetch transactions directly from a financial institution via OAuth.

**Strengths:** authoritative, structured, real-time.
**Weaknesses:** requires a separate OAuth integration for every institution; many banks and billers are not covered; data is limited to what the bank exposes (no invoice details, no vendor metadata beyond a merchant name).

This approach is complementary to email extraction, not a replacement — not every financial event goes through a bank API.

### Template-Based Email Parsers (Parseur, Docparser)

[Parseur](https://parseur.com/) and [Docparser](https://docparser.com/) are commercial products that use point-and-click template editors. A human opens a sample email, draws boxes around fields, names them, and the product applies that template to future emails.

**Strengths:** very accurate once a template is set up; no ML needed per-email.
**Weaknesses:** templates are created manually; they break when senders update their email design; scaling to hundreds of senders is labour-intensive.

Our approach automates exactly the template-creation step these products leave to humans.

### LLM Extraction (GPT-4, Claude, Gemini per-email)

The naive modern approach: send each email to a capable LLM with a prompt like "extract the amount, date, and vendor from this email and return JSON."

**Strengths:** works immediately with no setup; handles a wide variety of layouts.
**Weaknesses:** expensive at scale (every email costs a token budget); results are inconsistent across runs; field names and formats vary unless you invest heavily in prompt engineering; no guarantee the output matches a typed schema; the LLM has no knowledge of what "normal" looks like for a given sender.

### ML Document Parsers (Amazon Textract, Google Document AI)

These services are designed for scanned PDFs and images — invoices photographed or exported from accounting software. They use computer-vision models to identify tables, key-value pairs, and layout regions.

**Strengths:** excellent for PDFs and images with dense tabular data.
**Weaknesses:** overkill and expensive for plain-text HTML emails; require cloud API calls per document; not designed for sender-specific templates.

---

## Our Approach: Reverse-Template Extraction

Dwata's extractor works in five stages. The LLM is involved in exactly **one** of them — and only once per sender, not once per email.

### Stage 1: Cluster Similar Emails from the Same Sender

Not every email from a sender follows the same template. A bank might send monthly statements, fraud alerts, and promotional offers — three very different layouts. Mixing them would produce a useless template.

We load up to 200 recent emails from the sender and run a greedy multi-cluster grouping pass:

- Each email is represented as a flat string of its subject and body tokens.
- Emails are processed in order. Each one is compared (using normalised word-level edit distance) to the seed of every existing cluster.
- If the closest cluster is within a configurable threshold, the email joins it. Otherwise it starts a new cluster.
- As an efficiency guard: if after scanning one third of the available emails every cluster is still a singleton (no two emails are similar), we bail out early — there is no repeating template here.
- At the end, the cluster with the most members wins.

This is a meaningful improvement over a fixed-seed approach (comparing every email against email #1), because if the most recent email is a promotional one-off, a fixed-seed approach poisons all downstream comparisons.

### Stage 2: Build the Template by Diffing

With a cluster of similar emails in hand, we diff them at the word level to find what is constant across all emails and what varies.

The algorithm processes the cluster line by line and token by token:

- A token that appears in the same position across a configurable fraction of emails (the *support threshold*) is kept as literal text.
- A run of tokens that falls below the support threshold is collapsed into a single placeholder: `{{ placeholder_3 }}`.
- Subject lines go through the same process, producing something like `{{ subject_1 }} - Your bill for {{ subject_2}}`.

The support threshold is derived from the cluster size: larger clusters can afford stricter thresholds (e.g. 80%), while small clusters are more lenient (50%). This produces a template like:

```
Subject: Your bill for {{ subject_1 }}
---
Dear {{ placeholder_1 }},

Your bill amount of {{ placeholder_2 }} is due on {{ placeholder_3 }}.

Account number: {{ placeholder_4 }}
Thank you for being a valued customer.
```

The template encodes the structure of this sender's emails with no LLM involvement whatsoever.

### Stage 3: LLM Labels the Template Type

Here is where the LLM earns its single call.

We send the full template to a language model and ask it to classify the document: is this a **bill** (you owe money), a **transaction confirmation** (payment was made), both, or neither?

This is a cheap classification call on a short, clean, de-personalised text (no real amounts, names, or account numbers — just a template). The model returns a structured label: `has_bill`, `has_transaction`, `doc_type`.

### Stage 4: LLM Translates Placeholder Names

Given the label, we make one more LLM call (or two, if the document is both bill and transaction) to translate the opaque placeholder names into semantic field names.

For a bill, placeholders are mapped to: `total-amount`, `currency`, `issued-date`, `due-date`, `billing-period-start`, `billing-period-end`, `document-reference`, `service-identifier`.

For a transaction, they map to: `amount`, `currency`, `transaction-date`, `vendor`, `transaction-reference`.

The LLM sees only the template, not any real email. Output:

```
placeholder_2 → total-amount
placeholder_3 → due-date
placeholder_4 → service-identifier (account-number)
subject_1     → billing-period-start
```

We then render a *translated template* for human review:

```
Subject: Your bill for {{ billing-period-start }}
---
Dear {{ placeholder_1 }},

Your bill amount of {{ total-amount }} is due on {{ due-date }}.

Account number: {{ service-identifier }}
Thank you for being a valued customer.
```

Unmapped placeholders (like `placeholder_1`, the customer name, which is not a financial field) are left as-is.

### Stage 5: Apply the Template — No LLM

With the translated template in hand, we extract values from individual emails using **pure string matching** — no model calls.

For each template line that contains a placeholder, we use the surrounding fixed text as delimiters:

```
Template: Your bill amount of {{ total-amount }} is due on {{ due-date }}.
Email:    Your bill amount of ₹1,234.56 is due on 15 Feb 2025.

Extracted: total-amount = "₹1,234.56"
           due-date     = "15 Feb 2025"
```

The fixed anchors (`Your bill amount of`, `is due on`, `.`) bracket each variable region. Because consecutive varying tokens were already collapsed into a single placeholder during template building, there is no ambiguity about where one value ends and the next begins.

We apply this across the 10 most recent emails from the sender and display the results as a table — no LLM involved in any per-email processing.

---

## LLM Cost Profile

| Stage | LLM involved? | Frequency |
|---|---|---|
| Cluster emails | No | — |
| Build template | No | — |
| Label template type | **Yes** | Once per sender |
| Translate placeholders | **Yes** | Once per sender (up to 2 calls) |
| Extract values per email | No | — |

After the two one-time setup calls per sender, every subsequent email is parsed with zero LLM cost. For a sender with 500 emails, the total LLM work is the same as for a sender with 5 emails.

---

## Design Principles

The extractor is built around a single guiding idea from our internal design document: **parse into trusted types, not parse-then-validate** — inspired by Alexis King's [Parse, don't validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/).

This means:

- If extraction succeeds, the result is already a valid domain object — not a bag of strings awaiting cleanup.
- No partially-valid results. No confidence scores patching over ambiguity. If a required field cannot be mapped, extraction fails cleanly.
- Unknowns are represented explicitly in types (e.g. `Unknown` as a typed party, not a missing field).

The type system is the contract. The template pipeline is what gets us there efficiently.

---

## What Is Still Being Built

This extractor is functional and produces useful output today, but the full pipeline is not complete:

- **Typed output**: the current tool prints extracted values to a table; wiring this to the `FinancialTransaction` domain model (with typed `payer`, `payee`, amounts, and dates) is in progress.
- **User review queue**: ambiguous extractions (unmapped placeholders, multiple candidate matches) should surface in a UI for human confirmation rather than silently failing.
- **Vendor resolution**: mapping a vendor name string to a `KnownVendorId` or `CandidateVendorId` requires a separate lookup layer.
- **Template versioning**: when a sender updates their email layout, the stored template becomes stale. Automatic re-clustering and re-labelling on mismatch detection is planned.
- **Multi-template senders**: some senders use genuinely distinct templates (statements vs. alerts). The clustering step isolates these, but we currently use only the largest cluster. Running parallel templates per sender is on the roadmap.

---

## How to Try It

The extractor runs as a command-line tool. You will need:

- Rust toolchain installed
- Dwata running with at least one email account synced (so emails are in the local SQLite database)
- An LLM API key configured in `api.toml` (Gemini, OpenAI, or a local Ollama model)

To run from source, follow [docs/08-run-from-source.md](./08-run-from-source.md) first to clone the repositories and start the API so emails are synced to the local database.

Then run the extractor:

```bash
cargo run -p dwata-agents --bin template_based_financial_extractor -- \
  --email-from sender@example.com
```

Example with options:

```bash
# Use OpenAI instead of Gemini, raise the similarity threshold
cargo run -p dwata-agents --bin template_based_financial_extractor -- \
  --email-from billing@electricity-provider.com \
  --provider openai \
  --word-distance-threshold 0.45

# Skip the LLM steps entirely and inspect the raw generated template
cargo run -p dwata-agents --bin template_based_financial_extractor -- \
  --email-from billing@electricity-provider.com \
  --template-only
```

The tool will print:

1. How many emails it found and which cluster won
2. The raw generated template (with generic placeholder names)
3. The LLM's document type label (`bill`, `transaction`, or both)
4. The placeholder-to-field mappings from the LLM
5. The translated template (with semantic field names)
6. A table of extracted values from the 10 most recent emails

**Available flags:**

| Flag | Default | Description |
|---|---|---|
| `--email-from` | *(required)* | Sender address to scan |
| `--max-db-emails` | `200` | How many emails to load from the database |
| `--word-distance-threshold` | `0.35` | Similarity threshold (0 = identical, 1 = completely different) |
| `--provider` | `gemini` | LLM provider: `gemini`, `openai`, or `ollama` |
| `--model` | provider default | Override the model ID |
| `--template-only` | `false` | Stop after generating the template, skip LLM steps |

---

## Source and Further Reading

- Source code: [`dwata-agents/src/bin/template_based_financial_extractor.rs`](../dwata-agents/src/bin/template_based_financial_extractor.rs)
- Extraction design principles: [`docs/07-type-driven-financial-extraction.md`](./07-type-driven-financial-extraction.md)
- Running from source: [`docs/08-run-from-source.md`](./08-run-from-source.md)
- Project repository: [github.com/brainless/dwata](https://github.com/brainless/dwata)
