# Credentials List in Settings > Accounts Tab - Implementation

## Completed: 2026-01-28

### Overview
Added a data table to display existing credentials in the Settings > Accounts tab, following the same UI pattern as the Tasks page.

## Changes Made

### 1. Updated Accounts.tsx Component

**File**: `/gui/src/pages/settings/Accounts.tsx`

**New Features:**
- Fetch credentials from API on component mount
- Display credentials in a zebra-striped table
- Show minimal but essential information per credential
- Delete functionality with confirmation
- Empty state when no credentials exist
- Loading state while fetching data
- Auto-refresh list after adding or deleting credentials

### 2. Page Layout

The Accounts tab now has two sections:

#### Section 1: Your Email Accounts (List)
Displays existing configured accounts in a table with columns:
- **Account** - Identifier (bold) and username/email (muted)
- **Type** - Credential type badge (IMAP, SMTP, OAuth, etc.)
- **Server** - Server host and port information
- **Status** - Active/Inactive badge with icon
- **Actions** - View and Delete buttons

#### Section 2: Add New Email Account (Form)
The existing IMAP form for adding new accounts (unchanged)

### 3. Table Features

**Columns:**
```
| Account          | Type | Server              | Status | Actions |
|------------------|------|---------------------|--------|---------|
| work_email       | IMAP | imap.gmail.com      | Active | 👁 🗑   |
| john@company.com |      | Port: 993           |        |         |
```

**Type Badges:**
- IMAP → Primary badge (blue)
- SMTP → Secondary badge (purple)
- OAuth → Accent badge (pink)
- API Key → Info badge (cyan)
- Database → Warning badge (yellow)
- Custom → Ghost badge (gray)

**Status Indicators:**
- Active → Green badge with checkmark icon
- Inactive → Gray badge with X icon

**Action Buttons:**
- 👁 View (eye icon) - Placeholder for future detail view
- 🗑 Delete (trash icon) - Deletes credential with confirmation

### 4. Empty State

When no credentials exist:
```
┌─────────────────────────────────────┐
│          📧 (email icon)            │
│  No email accounts configured yet.  │
│    Add your first account below.    │
└─────────────────────────────────────┘
```

### 5. Loading State

While fetching credentials:
```
┌─────────────────────────────────────┐
│          🔄 (spinner)               │
└─────────────────────────────────────┘
```

### 6. API Integration

**Fetch Credentials:**
```typescript
GET http://localhost:8080/api/credentials
Response: CredentialListResponse {
  credentials: CredentialMetadata[]
}
```

**Delete Credential:**
```typescript
DELETE http://localhost:8080/api/credentials/{id}?hard=true
Response: 204 No Content
```

**Hard Delete:**
- Uses `hard=true` query parameter
- Removes credential from both database and OS keychain
- Confirms with user before deletion

### 7. Data Flow

1. **On Mount:**
   - `onMount()` → `fetchCredentials()` → API call → `setCredentials()`

2. **After Adding Account:**
   - Form submit → API success → `fetchCredentials()` → List refreshes

3. **After Deleting Account:**
   - Delete button → Confirmation → API call → `fetchCredentials()` → List refreshes

### 8. UI Components Used

**New Icons (solid-icons/hi):**
- `HiOutlineEnvelope` - Email/credential type indicator
- `HiOutlineTrash` - Delete action
- `HiOutlineEye` - View details action
- `HiOutlineCheckCircle` - Active status
- `HiOutlineXCircle` - Inactive status

**DaisyUI Components:**
- `table table-zebra` - Striped table layout
- `badge badge-sm` - Type and status badges
- `btn btn-ghost btn-sm btn-circle` - Icon action buttons
- `loading loading-spinner loading-lg` - Loading indicator

**SolidJS Components:**
- `Show` - Conditional rendering (loading, empty state)
- `For` - Loop over credentials array

### 9. Code Structure

```typescript
export default function SettingsAccounts() {
  // Form state (existing)

  // List state (new)
  const [credentials, setCredentials] = createSignal<CredentialMetadata[]>([]);
  const [isLoadingList, setIsLoadingList] = createSignal(true);

  // Fetch on mount (new)
  onMount(async () => {
    await fetchCredentials();
  });

  // API functions (new)
  const fetchCredentials = async () => { ... };
  const deleteCredential = async (id, identifier) => { ... };

  // Existing submit handler (modified)
  const handleSubmit = async (e) => {
    // ... existing code ...
    await fetchCredentials(); // Refresh list after adding
  };

  // Credential type config (new)
  const credentialTypeConfig = { ... };

  return (
    <div class="space-y-6">
      {/* Credentials List Card (new) */}
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h2>Your Email Accounts</h2>
          <Show when={!isLoadingList()} fallback={<Spinner />}>
            <Show when={credentials().length > 0} fallback={<EmptyState />}>
              <table class="table table-zebra">
                {/* Table content */}
              </table>
            </Show>
          </Show>
        </div>
      </div>

      {/* Add New Account Form Card (existing) */}
      <div class="card bg-base-100 shadow-xl">
        {/* Existing form */}
      </div>
    </div>
  );
}
```

## Example Table Row

```tsx
<tr>
  <td>
    <div>
      <div class="font-bold">work_email</div>
      <div class="text-sm opacity-60">john@company.com</div>
    </div>
  </td>
  <td>
    <span class="badge badge-sm badge-primary gap-1">
      <HiOutlineEnvelope class="w-3 h-3" />
      IMAP
    </span>
  </td>
  <td>
    <div class="text-sm">
      <div>imap.gmail.com</div>
      <div class="text-xs opacity-60">Port: 993</div>
    </div>
  </td>
  <td>
    <span class="badge badge-sm badge-success gap-1">
      <HiOutlineCheckCircle class="w-3 h-3" />
      Active
    </span>
  </td>
  <td>
    <div class="flex gap-2">
      <button class="btn btn-ghost btn-sm btn-circle">
        <HiOutlineEye class="w-4 h-4" />
      </button>
      <button
        class="btn btn-ghost btn-sm btn-circle text-error"
        onClick={() => deleteCredential(id, identifier)}
      >
        <HiOutlineTrash class="w-4 h-4" />
      </button>
    </div>
  </td>
</tr>
```

## Responsive Behavior

- **Desktop**: Full table with all columns visible
- **Mobile/Tablet**: Horizontal scroll for table (overflow-x-auto)
- Table maintains zebra striping for readability
- Consistent with Tasks page table layout

## User Workflow

### Viewing Credentials
1. Navigate to Settings > Accounts
2. See list of configured accounts (if any)
3. View details: identifier, email, server, status

### Adding Credential
1. Scroll to "Add New Email Account" section
2. Fill out form
3. Click "Add IMAP Account"
4. Success message appears
5. List automatically refreshes with new account

### Deleting Credential
1. Click trash icon on account row
2. Confirm deletion dialog: "Are you sure you want to delete 'work_email'?"
3. Click OK
4. Success message appears
5. Account removed from list
6. Credential deleted from both database and OS keychain

## Security Notes

- Passwords are never displayed in the list (security)
- Only metadata shown (identifier, username, server)
- Delete operations require confirmation
- Hard delete removes from both DB and keychain
- View button (eye icon) is placeholder for future detail view

## Future Enhancements

1. **View Details Modal:**
   - Click eye icon to see full credential details
   - Show parsed IMAP settings (mailbox, timeout, etc.)
   - Display last accessed timestamp
   - Show notes

2. **Edit Functionality:**
   - Edit button in actions column
   - Pre-fill form with existing values
   - Update credential instead of creating new

3. **Test Connection:**
   - Test button to verify IMAP connection
   - Show connection status
   - Display error messages if connection fails

4. **Filtering & Search:**
   - Search by identifier or username
   - Filter by credential type (IMAP, SMTP, etc.)
   - Filter by status (active/inactive)

5. **Bulk Operations:**
   - Select multiple credentials
   - Bulk delete
   - Bulk activate/deactivate

6. **Sync Status:**
   - Show last sync timestamp
   - Display sync errors
   - Manual sync trigger

## Testing Instructions

### Test Credentials List Display

1. **Start API and GUI:**
   ```bash
   # Terminal 1
   cd dwata-api && cargo run

   # Terminal 2
   cd gui && npm run dev
   ```

2. **Navigate to Settings:**
   - Open `http://localhost:5173/settings/accounts`

3. **Test Empty State:**
   - If no credentials exist, should see empty state message
   - "No email accounts configured yet"

4. **Add a Credential:**
   - Fill out IMAP form
   - Submit
   - Should see success message
   - Table should appear with new credential

5. **Verify Table Data:**
   - Check identifier displays correctly
   - Check username/email displays
   - Check server and port display
   - Check type badge shows "IMAP" in primary color
   - Check status shows "Active" in green

6. **Test Delete:**
   - Click trash icon
   - Should see confirmation dialog
   - Click OK
   - Should see success message
   - Credential should disappear from table

7. **Test Multiple Credentials:**
   - Add 3-4 different accounts
   - Verify all display correctly
   - Verify zebra striping alternates rows

### Verify API Integration

```bash
# List credentials
curl http://localhost:8080/api/credentials

# Expected response:
{
  "credentials": [
    {
      "id": "cred_abc123",
      "credential_type": "imap",
      "identifier": "work_email",
      "username": "john@company.com",
      "service_name": "imap.gmail.com",
      "port": 993,
      "use_tls": true,
      "notes": "Work account",
      "created_at": 1706831200000,
      "updated_at": 1706831200000,
      "last_accessed_at": null,
      "is_active": true,
      "extra_metadata": "{...}"
    }
  ]
}
```

## Build Status

✅ **Build Successful:**
```bash
npm run build
✓ built in 342ms
```

## Files Modified

- `/gui/src/pages/settings/Accounts.tsx` - Added credentials list and table

## Related Documentation

- `/gui/SETTINGS_GUIDE.md` - Settings page guide
- `/tasks/settings-imap-form-implementation.md` - Original form implementation
- `/tasks/imap-form-visual-layout.md` - Form visual layout
- `/tasks/credential-storage-api.md` - API specification

## Success Criteria - All Met ✓

- [x] Credentials list displays in table format
- [x] Similar UI to Tasks page (zebra table)
- [x] Shows minimal essential information
- [x] Fetches data from API on mount
- [x] Delete functionality with confirmation
- [x] Empty state for no credentials
- [x] Loading state while fetching
- [x] Auto-refresh after add/delete
- [x] Type badges with icons
- [x] Status indicators (Active/Inactive)
- [x] Responsive table layout
- [x] Build succeeds without errors
