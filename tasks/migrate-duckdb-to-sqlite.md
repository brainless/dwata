# Task: Migrate from DuckDB to SQLite

**Status**: Pending
**Priority**: Medium
**Created**: 2026-02-02
**Estimated Effort**: 4-6 hours
**Risk Level**: Low

## Objective

Migrate the dwata project from DuckDB to SQLite as the embedded database engine. This migration involves replacing the database crate dependency, updating the schema to use SQLite's auto-increment mechanism, and updating all database query code to use the rusqlite API.

## Background

The project currently uses DuckDB 1.1 with bundled features as its embedded database. While DuckDB is powerful for analytics, SQLite is more widely used, better supported, has better foreign key support, and is more appropriate for the application's use case.

Key issues with current DuckDB implementation:
- Foreign key constraints were disabled due to DuckDB bugs (see migrations.rs line 799-803)
- Less familiar to potential contributors
- Larger footprint than needed for the application

**Solution**: Replace DuckDB with SQLite, which provides better reliability for transactional workloads and foreign key support.

## Benefits

1. **Better Foreign Key Support** - Can re-enable foreign key constraints that were disabled due to DuckDB issues
2. **Wider Ecosystem** - More tooling, better documentation, larger community
3. **Smaller Footprint** - More lightweight than DuckDB for the application's needs
4. **Better Known** - Easier for contributors to work with familiar technology
5. **Native Support** - Better platform compatibility and tooling support
6. **Proven Reliability** - Battle-tested for transactional workloads

## Migration Scope

### Files to Modify: 17 total

| File | Changes | Complexity |
|------|---------|-----------|
| `Cargo.toml` | Replace `duckdb` with `rusqlite` | Trivial |
| `dwata-api/src/database/migrations.rs` | Remove sequences, use AUTOINCREMENT | Moderate |
| `dwata-api/src/database/mod.rs` | Update imports and connection types | Low |
| `dwata-api/src/database/credentials.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/emails.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/downloads.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/extraction_jobs.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/events.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/contacts.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/companies.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/positions.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/contact_links.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/linkedin_connections.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/financial_transactions.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/financial_extraction_sources.rs` | Update imports and params macro | Low |
| `dwata-api/src/database/financial_patterns.rs` | Update imports and params macro | Low |
| `dwata-api/src/helpers/database.rs` | Update file extension and initialization | Trivial |

### Code Changes Summary

**DuckDB References**: 43 occurrences across 16 files
**Sequence Usage**: 19 CREATE SEQUENCE statements + 19 nextval() calls
**RETURNING Clauses**: 14 INSERT...RETURNING statements (compatible with SQLite)

## Implementation Phases

### Phase 1: Update Dependencies

**File**: `Cargo.toml`

**Changes**:
```toml
# BEFORE:
duckdb = { version = "1.1", features = ["bundled"] }

# AFTER:
rusqlite = { version = "0.31", features = ["bundled"] }
```

**Notes**:
- Both crates support bundled SQLite/DuckDB binaries
- No other dependency changes needed

### Phase 2: Rewrite Database Schema

**File**: `dwata-api/src/database/migrations.rs`

**Key Changes**:
1. Remove all 19 `CREATE SEQUENCE` statements
2. Replace `DEFAULT nextval('seq_*')` with `AUTOINCREMENT` in all table definitions
3. Update imports from `duckdb::Connection` to `rusqlite::Connection`
4. Update error handling for rusqlite types

**Before**:
```sql
CREATE SEQUENCE IF NOT EXISTS seq_emails_id;

CREATE TABLE IF NOT EXISTS emails (
    id INTEGER PRIMARY KEY DEFAULT nextval('seq_emails_id'),
    ...
);
```

**After**:
```sql
CREATE TABLE IF NOT EXISTS emails (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ...
);
```

**Tables to Update** (19 total):
- agent_sessions
- agent_messages
- agent_tool_calls
- credentials_metadata
- download_jobs
- download_items
- emails
- email_attachments
- extraction_jobs
- events
- contacts
- companies
- contact_links
- positions
- linkedin_connections
- financial_transactions
- financial_extraction_sources
- financial_patterns

**Additional Changes**:
- Keep all indexes unchanged (compatible syntax)
- Keep all UNIQUE constraints unchanged
- Can optionally re-enable foreign key constraints (PRAGMA foreign_keys = ON)
- Update seed data INSERT for financial_patterns (compatible with SQLite)

### Phase 3: Update Database Connection Module

**File**: `dwata-api/src/database/mod.rs`

**Changes**:
```rust
// BEFORE:
use duckdb::{params, Connection};

// AFTER:
use rusqlite::{params, Connection};
```

**Function Updates**:
- `Database::new()` - Update Connection::open() call (same API)
- All query methods - Update params! macro usage (same syntax)
- Error handling - Update to use rusqlite::Error types

**Notes**:
- Connection API is nearly identical between duckdb and rusqlite
- params! macro has same syntax
- query_row(), prepare(), execute() methods work the same way

### Phase 4: Update Database Query Files

**Files**: All `dwata-api/src/database/*.rs` files (14 query modules)

**Pattern to Apply**:

```rust
// BEFORE:
use duckdb::Connection;
use duckdb::params;

pub async fn insert_credential(...) -> Result<i64, ...> {
    let id: i64 = conn.query_row(
        "INSERT INTO ... RETURNING id",
        duckdb::params![...],
        |row| row.get(0),
    )?;
}

// AFTER:
use rusqlite::Connection;
use rusqlite::params;

pub async fn insert_credential(...) -> Result<i64, ...> {
    let id: i64 = conn.query_row(
        "INSERT INTO ... RETURNING id",
        rusqlite::params![...],
        |row| row.get(0),
    )?;
}
```

**Files to Update**:
1. `credentials.rs` (7 duckdb:: references)
2. `emails.rs` (3 references)
3. `downloads.rs` (11 references)
4. `extraction_jobs.rs` (1 reference)
5. `events.rs` (1 reference)
6. `contacts.rs` (1 reference)
7. `companies.rs` (2 references)
8. `positions.rs` (1 reference)
9. `contact_links.rs` (2 references)
10. `linkedin_connections.rs` (2 references)
11. `financial_transactions.rs` (1 reference)
12. `financial_extraction_sources.rs` (1 reference)
13. `financial_patterns.rs` (2 references)
14. `queries.rs` (3 references)

**Error Handling Updates**:
```rust
// BEFORE:
.map_err(|e| match e {
    duckdb::Error::QueryReturnedNoRows => CredentialDbError::NotFound,
    _ => CredentialDbError::DatabaseError(e.to_string()),
})

// AFTER:
.map_err(|e| match e {
    rusqlite::Error::QueryReturnedNoRows => CredentialDbError::NotFound,
    _ => CredentialDbError::DatabaseError(e.to_string()),
})
```

### Phase 5: Update Database Helper

**File**: `dwata-api/src/helpers/database.rs`

**Changes**:
1. Update database file extension:
   ```rust
   // BEFORE:
   let db_path = data_dir.join("dwata").join("db.duckdb");

   // AFTER:
   let db_path = data_dir.join("dwata").join("db.sqlite");
   ```

2. Update initialization function:
   ```rust
   // BEFORE:
   use duckdb::Connection;

   // AFTER:
   use rusqlite::Connection;
   ```

3. Optionally enable foreign keys:
   ```rust
   pub fn initialize_database() -> anyhow::Result<Database> {
       let db_path = get_db_path()?;
       let db = Database::new(&db_path)?;

       // Enable foreign key constraints (optional)
       {
           let conn = db.connection.lock().unwrap();
           conn.execute("PRAGMA foreign_keys = ON", [])?;
       }

       Ok(db)
   }
   ```

**Updated Paths**:
- **macOS**: `~/Library/Application Support/dwata/db.sqlite`
- **Linux**: `~/.local/share/dwata/db.sqlite`
- **Windows**: `%LOCALAPPDATA%\dwata\db.sqlite`

### Phase 6: Update Main Entry Point

**File**: `dwata-api/src/main.rs`

**Changes**:
- No significant changes needed
- Database initialization remains the same
- Print message will automatically reflect new path

### Phase 7: Testing

**Manual Testing Steps**:

1. **Delete old database**:
   ```bash
   # macOS
   rm ~/Library/Application\ Support/dwata/db.duckdb*
   ```

2. **Run server**:
   ```bash
   cd dwata-api
   cargo run
   ```

3. **Verify migrations**:
   ```bash
   sqlite3 ~/Library/Application\ Support/dwata/db.sqlite
   .tables
   .schema credentials_metadata
   SELECT COUNT(*) FROM financial_patterns;
   ```

4. **Test CRUD operations**:
   - Create credential via API
   - List credentials
   - Update credential
   - Delete credential

5. **Test download operations**:
   - Start email download
   - Check download status
   - Verify emails saved

6. **Test extraction operations**:
   - Run financial extraction
   - Verify transactions extracted
   - Check pattern match counts

7. **Test foreign keys** (if enabled):
   ```sql
   -- This should fail if foreign keys are enabled
   INSERT INTO emails (credential_id, ...) VALUES (99999, ...);
   ```

**Automated Testing**:

Create integration test file: `dwata-api/tests/sqlite_migration_test.rs`

```rust
#[cfg(test)]
mod tests {
    use dwata_api::database::Database;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.sqlite");

        let db = Database::new(&db_path).unwrap();

        // Verify schema exists
        let conn = db.connection.lock().unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"credentials_metadata".to_string()));
        assert!(tables.contains(&"emails".to_string()));
        assert!(tables.contains(&"financial_patterns".to_string()));
    }

    #[test]
    fn test_autoincrement_works() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.sqlite");
        let db = Database::new(&db_path).unwrap();

        // Insert without specifying ID
        let conn = db.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO financial_patterns (name, regex_pattern, document_type,
             status, confidence, amount_group, is_default, is_active, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                "test_pattern",
                "test_regex",
                "bill",
                "pending",
                0.9,
                1,
                false,
                true,
                0,
                0
            ],
        ).unwrap();

        // Verify ID was assigned
        let id: i64 = conn
            .query_row("SELECT id FROM financial_patterns WHERE name = 'test_pattern'", [], |row| row.get(0))
            .unwrap();

        assert!(id > 0);
    }

    #[test]
    fn test_default_patterns_seeded() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.sqlite");
        let db = Database::new(&db_path).unwrap();

        let conn = db.connection.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM financial_patterns", [], |row| row.get(0))
            .unwrap();

        assert!(count >= 15, "Should have at least 15 default patterns");
    }
}
```

## SQL Compatibility Notes

### Compatible Features (No Changes Needed)

✅ **Data Types**:
- INTEGER, VARCHAR, BOOLEAN, FLOAT, BIGINT, DOUBLE
- TEXT (for JSON storage)

✅ **SQL Features**:
- INSERT ... RETURNING id (SQLite 3.35+)
- CREATE INDEX IF NOT EXISTS
- UNIQUE constraints
- CHECK constraints
- Prepared statements with ? placeholders
- Transactions (BEGIN, COMMIT, ROLLBACK)

✅ **Query Patterns**:
- INSERT, SELECT, UPDATE, DELETE
- WHERE, ORDER BY, LIMIT, OFFSET
- Aggregate functions (COUNT, SUM, etc.)
- JOINs (though not currently used much)

### Differences Handled

🔄 **Auto-Increment**:
- DuckDB: `CREATE SEQUENCE` + `nextval()`
- SQLite: `AUTOINCREMENT` keyword

🔄 **Error Types**:
- DuckDB: `duckdb::Error`
- SQLite: `rusqlite::Error`

🔄 **Connection API**:
- Minor differences in error handling
- Same core API (Connection::open, prepare, query_row, execute)

### Foreign Keys (Optional Enhancement)

Can optionally re-enable foreign key constraints that were disabled due to DuckDB bugs:

```rust
// At database initialization
conn.execute("PRAGMA foreign_keys = ON", [])?;
```

Then add back foreign key constraints in schema:
```sql
CREATE TABLE emails (
    ...
    credential_id INTEGER NOT NULL,
    FOREIGN KEY (credential_id) REFERENCES credentials_metadata(id)
);
```

This is **optional** and can be done in a follow-up phase to avoid scope creep.

## Risk Assessment

**Overall Risk: LOW**

**Mitigating Factors**:
- ✅ No data migration needed (fresh install approach)
- ✅ SQL is standard and compatible
- ✅ APIs are nearly identical (duckdb crate vs rusqlite crate)
- ✅ RETURNING clause supported in both
- ✅ Can test thoroughly before deploying
- ✅ Simple rollback (revert commits)

**Potential Issues**:
- ⚠️ Edge cases in error handling
- ⚠️ Subtle differences in SQL dialect
- ⚠️ Type conversion edge cases

**Mitigation Strategy**:
- Comprehensive testing of all CRUD operations
- Test error paths and edge cases
- Monitor logs during initial deployment
- Keep DuckDB version in git history for easy rollback

## Success Criteria

**Must Have**:
- [ ] `Cargo.toml` updated with rusqlite dependency
- [ ] All 19 tables use AUTOINCREMENT instead of sequences
- [ ] All 16 database query files updated to use rusqlite
- [ ] Database file extension changed to `.sqlite`
- [ ] Server starts successfully with SQLite
- [ ] All migrations run without errors
- [ ] Credentials CRUD operations work
- [ ] Email download and storage works
- [ ] Financial extraction works
- [ ] Pattern management works
- [ ] All existing features functional
- [ ] No errors in server logs

**Nice to Have**:
- [ ] Foreign key constraints re-enabled
- [ ] Integration tests passing
- [ ] Performance benchmarks documented
- [ ] Updated documentation references

## File Checklist

### Core Changes (Required)
- [ ] `Cargo.toml` - Replace duckdb with rusqlite
- [ ] `dwata-api/src/database/migrations.rs` - Rewrite schema with AUTOINCREMENT
- [ ] `dwata-api/src/database/mod.rs` - Update imports and connection types
- [ ] `dwata-api/src/helpers/database.rs` - Update file extension and imports

### Database Query Modules (Required)
- [ ] `dwata-api/src/database/credentials.rs` - Update imports and params
- [ ] `dwata-api/src/database/emails.rs` - Update imports and params
- [ ] `dwata-api/src/database/downloads.rs` - Update imports and params
- [ ] `dwata-api/src/database/extraction_jobs.rs` - Update imports and params
- [ ] `dwata-api/src/database/events.rs` - Update imports and params
- [ ] `dwata-api/src/database/contacts.rs` - Update imports and params
- [ ] `dwata-api/src/database/companies.rs` - Update imports and params
- [ ] `dwata-api/src/database/positions.rs` - Update imports and params
- [ ] `dwata-api/src/database/contact_links.rs` - Update imports and params
- [ ] `dwata-api/src/database/linkedin_connections.rs` - Update imports and params
- [ ] `dwata-api/src/database/financial_transactions.rs` - Update imports and params
- [ ] `dwata-api/src/database/financial_extraction_sources.rs` - Update imports and params
- [ ] `dwata-api/src/database/financial_patterns.rs` - Update imports and params
- [ ] `dwata-api/src/database/queries.rs` - Update imports and params
- [ ] `dwata-api/src/database/models.rs` - Check for duckdb types (if any)

### Testing (Optional)
- [ ] `dwata-api/tests/sqlite_migration_test.rs` - Integration tests (NEW)

### Documentation (Optional)
- [ ] `DEVELOP.md` - Update database references
- [ ] `README.md` - Update if it mentions DuckDB
- [ ] `docs/03-database-schema.md` - Update if exists

## Migration Script

For convenience, here's a find-and-replace script:

```bash
#!/bin/bash
# Run from project root

echo "Migrating from DuckDB to SQLite..."

# Update imports in Rust files
find dwata-api/src/database -name "*.rs" -type f -exec sed -i '' 's/use duckdb::/use rusqlite::/g' {} +
find dwata-api/src/database -name "*.rs" -type f -exec sed -i '' 's/duckdb::params/rusqlite::params/g' {} +
find dwata-api/src/database -name "*.rs" -type f -exec sed -i '' 's/duckdb::Error/rusqlite::Error/g' {} +
find dwata-api/src/helpers -name "*.rs" -type f -exec sed -i '' 's/use duckdb::/use rusqlite::/g' {} +

echo "✓ Updated imports"
echo "! Manual steps required:"
echo "  1. Update Cargo.toml dependency"
echo "  2. Rewrite migrations.rs schema"
echo "  3. Test thoroughly"
```

**Note**: This script handles the mechanical replacements but manual review is required, especially for the schema migration.

## Timeline

**Estimated Duration**: 4-6 hours for an experienced Rust developer

- **Hour 1**: Update Cargo.toml, rewrite migrations.rs schema
- **Hour 2**: Update database/mod.rs and helper files
- **Hour 3-4**: Update all database query modules (14 files)
- **Hour 5**: Testing and debugging
- **Hour 6**: Edge cases and documentation

**Parallel Approach**: If multiple developers:
- Developer 1: Schema and core modules (migrations, mod.rs, helpers)
- Developer 2: Query modules (credentials, emails, downloads)
- Developer 3: Query modules (extraction, events, contacts, companies)
- Developer 4: Query modules (positions, financial_*)

This could reduce wall-clock time to 2-3 hours.

## Future Enhancements

**Post-Migration**:
1. **Re-enable Foreign Keys** - Add PRAGMA foreign_keys = ON and FK constraints
2. **Add Database Indexes** - Optimize frequently queried fields
3. **Connection Pooling** - Use r2d2 or similar for connection pooling
4. **Write-Ahead Logging** - Enable WAL mode for better concurrency
5. **Backup Strategy** - Implement automated SQLite backups
6. **Database Vacuum** - Schedule VACUUM operations for maintenance

**Monitoring**:
- Log query execution times
- Monitor database file size
- Track slow queries
- Alert on database errors

## References

- **rusqlite Documentation**: https://docs.rs/rusqlite/
- **SQLite AUTOINCREMENT**: https://www.sqlite.org/autoinc.html
- **SQLite Foreign Keys**: https://www.sqlite.org/foreignkeys.html
- **SQLite RETURNING**: https://www.sqlite.org/lang_returning.html

## Notes

- No data migration needed (confirmed by user)
- Users will need to delete old `db.duckdb` file and recreate database
- Consider creating a migration guide for users if application is already in use
- The change from `.duckdb` to `.sqlite` extension is cosmetic but helpful for tooling
