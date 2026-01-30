# Extractors Crate

A reusable extraction library for extracting structured entities from email data.

## Architecture

This crate implements various extraction methods while keeping the type definitions separate in the `shared-types` crate. This design allows:

- **Reusability**: Other projects can use this crate with their own entity types
- **Flexibility**: Choose which extractors to use based on your needs
- **Clean dependencies**: Heavy ML dependencies are isolated here

## Available Extractors

### AttachmentParserExtractor

Extracts entities from structured email attachments with high confidence (0.95+).

**Supported formats:**
- **ICS (iCalendar)**: Calendar events from meeting invitations
- **VCF (vCard)**: Contact information

**Example:**

```rust
use extractors::{AttachmentParserExtractor, IcsParserConfig, TimezoneHandling};
use shared_types::{Extractor, ExtractionInput};

// Create with default configuration
let extractor = AttachmentParserExtractor::with_defaults();

// Or customize ICS parsing
let extractor = AttachmentParserExtractor::new(IcsParserConfig {
    extract_attendees: true,
    extract_location: true,
    extract_recurrence: false,
    timezone_handling: TimezoneHandling::ConvertToUser,
});

// Extract from email
let results = extractor.extract(&input)?;

for result in results {
    println!("Extracted: {:?}", result.entity);
    println!("Confidence: {:.2}", result.confidence);
    println!("Method: {:?}", result.method);
}
```

## Extractor Trait

All extractors implement the `Extractor` trait from `shared-types`:

```rust
pub trait Extractor {
    fn extract(&self, input: &ExtractionInput) -> Result<Vec<ExtractionResult>, ExtractionError>;
    fn data_type(&self) -> DataType;
    fn method(&self) -> ExtractionMethod;
    fn version(&self) -> String;
}
```

## Entity Types

Extractors return entities defined in `shared-types`:

- `ExtractedProject`: Project information
- `ExtractedTask`: Task/todo items
- `ExtractedEvent`: Calendar events
- `ExtractedContact`: Contact information
- `ExtractedLocation`: Location data

Each extraction result includes:
- **Entity data**: The extracted information
- **Confidence score**: Overall and per-field confidence
- **Evidence**: Text spans showing where the data was found
- **Relationships**: Links to other extracted entities
- **Provenance**: Extraction method, version, and timestamp

## Future Extractors

Planned extractors (see `tasks/` directory):

- **PatternBasedExtractor**: Regex and keyword matching for structured text
- **GlinerExtractor**: Named Entity Recognition using GLiNER models
- **LLMExtractor**: LLM-based extraction for complex scenarios

## Testing

Run tests:

```bash
cargo test -p extractors
```

Run with output:

```bash
cargo test -p extractors -- --nocapture
```

## Dependencies

- `shared-types`: Type definitions and trait interface
- `ical`: ICS (iCalendar) parsing
- `chrono`: Date/time handling
- `serde`: Serialization
- `anyhow`: Error handling

## Usage in Other Projects

Add to your `Cargo.toml`:

```toml
[dependencies]
extractors = { path = "../path/to/extractors" }
shared-types = { path = "../path/to/shared-types" }
```

Then use the extractors:

```rust
use extractors::{AttachmentParserExtractor, Extractor};
use shared_types::ExtractionInput;

let extractor = AttachmentParserExtractor::with_defaults();
let results = extractor.extract(&input)?;
```
