# Full-Text Search (FTS) Implementation

## Overview
Added SQLite FTS5-powered email search to the Dwata application, with field-specific search support and smart exclusion of HTML content.

## Features

### Search Capabilities
- **Full-text search** across email content (subject, body text, sender)
- **Prefix matching**: Automatically enabled for natural searching
  - `hdfc` matches `hdfcbank`, `hdfclife`, etc.
  - `ama` matches `amazon`, `amazonfresh`, etc.
- **Field-specific search** using prefixes:
  - `from:example.com` - Search only sender addresses
  - `subject:invoice` - Search only subject lines
  - `body:payment` - Search only email body text
- **Smart filtering**: Automatically excludes HTML body from general searches (reduces noise)
- **Context-aware**: Works with folder, label, and account filters

### Search Syntax
```
# General search (searches subject, body_text, and from_address)
invoice payment

# Prefix matching (automatic)
hdfc          # matches hdfcbank, hdfclife, etc.
ama           # matches amazon, amazonfresh, etc.

# Search by sender
from:example.com
from:hdfc      # matches from:hdfcbank, from:hdfclife, etc.
from:john      # matches john@company.com, johnny@example.com, etc.

# Search by subject
subject:meeting
subject:"quarterly report"

# Search by body content
body:urgent

# Combined search (automatic field-specific query)
from:amazon subject:order
```

## Implementation Details

### Backend Changes

#### 1. Database Function (`dwata-api/src/database/emails.rs`)
- Added `list_emails_fts()` function
- Parses user query for field-specific searches
- Converts user-friendly syntax (`from:`) to FTS5 column syntax (`from_address:`)
- Default search excludes `body_html` (only searches `subject`, `body_text`, `from_address`)

#### 2. API Handler (`dwata-api/src/handlers/emails.rs`)
- Modified `list_emails()` to check for `search_query` parameter
- Routes to FTS function when search query is provided
- Falls back to regular listing when no search query

### Frontend Changes

#### 1. API Client (`gui/src/api/emails.ts`)
- Updated all fetch functions to accept optional `searchQuery` parameter
- `fetchEmailsByFolder()`, `fetchEmailsByLabel()`, `fetchEmailsByAccount()`
- Automatically appends search query to API requests

#### 2. UI Component (`gui/src/pages/Emails.tsx`)
- Added search input with helpful placeholder text
- Separated search input state from active search state
- Search triggered by:
  - Clicking "Search" button
  - Pressing Enter in search box
- Added "Clear Search" button (✕) when search is active
- Reactive search: Results update automatically when search changes

## Database Schema

### FTS5 Virtual Table
```sql
CREATE VIRTUAL TABLE emails_fts USING fts5(
    subject,
    body_text,
    body_html,
    from_address,
    content='emails',
    content_rowid='id'
)
```

### Triggers
- `emails_fts_ai` - Auto-insert on email insert
- `emails_fts_ad` - Auto-delete on email delete
- `emails_fts_au` - Auto-update on email update

## Usage Examples

### Basic Search
1. Type search query in the search box (e.g., "invoice")
2. Click "Search" or press Enter
3. Results show emails matching the query
4. Click ✕ to clear search and return to normal view

### Advanced Search
```
# Find emails from Amazon containing "order"
from:amazon.com order

# Find emails with "invoice" in subject
subject:invoice

# Find emails from a specific sender with specific subject
from:finance@company.com subject:report
```

## Performance Considerations
- FTS5 provides fast full-text search even with thousands of emails
- Indexes are automatically maintained via triggers
- Search query parsing is minimal overhead
- HTML content exclusion reduces false positives and speeds up searches

## Future Enhancements
- Add date range filters (e.g., `after:2024-01-01`)
- Support for negative searches (e.g., `-from:spam.com`)
- Save frequently used searches
- Search result highlighting
- Fuzzy matching for typos
