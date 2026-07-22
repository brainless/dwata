# Dwata Documentation

This docs set reflects the current product focus: local email ingestion + reverse-template financial extraction.

## Reading Order

1. [01-product-overview.md](./01-product-overview.md) - Current product scope and boundaries
2. [02-current-architecture.md](./02-current-architecture.md) - Runtime/data-flow architecture for this branch
3. [03-type-driven-financial-extraction.md](./03-type-driven-financial-extraction.md) - Extraction design contract
4. [04-run-from-source.md](./04-run-from-source.md) - Run the supported backend from source
5. [05-blog-financial-email-extraction.md](./05-blog-financial-email-extraction.md) - Deep dive on reverse-template extraction

## Notes

- Old docs `01` to `06` were removed because they no longer matched the current implementation focus.
- The most reliable signal for active product surface is the migration chain in `dwata-api/migrations/` plus recent commits.
