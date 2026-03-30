# Extracting Financial Data from Emails Without Reading Every Email with an LLM

> **Note:** This document describes the original template-based extraction approach. The current system uses a Knowledge Graph extraction pipeline described in [`docs/06-knowledge-graph-extraction.md`](./06-knowledge-graph-extraction.md).
>
> The template-based approach described here has been removed from the codebase in favor of the KG extraction agent, which provides better accuracy on small models and incremental entity linking through BM25 pre-population.

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

Our original approach attempted to automate exactly the template-creation step these products leave to humans. This has since been replaced by the Knowledge Graph extraction pipeline.

### LLM Extraction (GPT-4, Claude, Gemini per-email)

The naive modern approach: send each email to a capable LLM with a prompt like "extract the amount, date, and vendor from this email and return JSON."

**Strengths:** works on any email format without setup.
**Weaknesses:** expensive at scale (per-token pricing adds up on thousands of emails); accuracy varies with prompt phrasing and model version; no built-in vendor normalization or duplicate detection.

---

## The Current Approach: Knowledge Graph Extraction

The system now uses a **Knowledge Graph extraction pipeline** that extracts structured entities from emails into a personal Knowledge Graph. This approach is documented in [`docs/06-knowledge-graph-extraction.md`](./06-knowledge-graph-extraction.md).

### Key Features of the New System

- **Four-pass sequential extraction**: Identity Resolution → Financial Extraction → Event Extraction → Order Extraction
- **BM25 pre-population**: Before each pass, existing entities are injected into the prompt so the LLM can link to them
- **Document labeler**: Determines which passes to run based on email content
- **Small model compatibility**: Designed to work with models < 7B parameters
- **Incremental extraction**: Entities are persisted and indexed after each pass

### Entity Types

The system extracts these financial entity types:
- `bill` — Invoices, utility bills, subscription charges
- `transaction` — Bank debits, credits, transfers  
- `subscription` — Recurring service subscriptions
- `order` — E-commerce purchase confirmations

---

## Historical Note: Template-Based Extraction

The template-based extraction system described in the original version of this document has been removed. It worked by:

1. Clustering emails from the same sender by similarity
2. Generating a positional template with placeholders
3. Using an LLM to name the variables semantically
4. Extracting values from future emails using the template

This approach was functional but had limitations:
- Templates became stale when senders updated their email format
- Required separate handling for each sender
- Difficult to link related entities (bills to transactions to subscriptions)

The Knowledge Graph approach solves these issues by maintaining persistent entities with BM25 search-based linking across all passes.

---

## Source and Further Reading

- **Current extraction system**: [`docs/06-knowledge-graph-extraction.md`](./06-knowledge-graph-extraction.md)
- Knowledge Graph extraction agent: `dwata-agents/src/kg_email_extractor/`
- XLSX bank statement extractor (still maintained): `dwata-agents/src/statement_extractor/`
- Project repository: [github.com/brainless/dwata](https://github.com/brainless/dwata)
