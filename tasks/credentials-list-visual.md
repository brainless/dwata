# Credentials List - Visual Layout

## Page Layout

```
┌───────────────────────────────────────────────────────────────────────┐
│  Settings                                                             │
│  [General] [API Keys] [Accounts]  ← Active tab                        │
│  ──────────────────────────────────────────────────────────────────   │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │ Your Email Accounts                                             │ │
│  │ Manage your configured email accounts                           │ │
│  │                                                                  │ │
│  │ ┌───────────────────────────────────────────────────────────┐  │ │
│  │ │ Account          │Type│Server           │Status │Actions │  │ │
│  │ ├───────────────────────────────────────────────────────────┤  │ │
│  │ │ work_email       │📧  │imap.gmail.com   │✓ Active│ 👁 🗑  │  │ │
│  │ │ john@company.com │IMAP│Port: 993        │        │        │  │ │
│  │ ├───────────────────────────────────────────────────────────┤  │ │
│  │ │ personal_gmail   │📧  │imap.gmail.com   │✓ Active│ 👁 🗑  │  │ │
│  │ │ me@gmail.com     │IMAP│Port: 993        │        │        │  │ │
│  │ ├───────────────────────────────────────────────────────────┤  │ │
│  │ │ old_account      │📧  │imap.oldmail.com │✗ Inact.│ 👁 🗑  │  │ │
│  │ │ old@mail.com     │IMAP│Port: 993        │        │        │  │ │
│  │ └───────────────────────────────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │ Add New Email Account                                           │ │
│  │ Add your IMAP email accounts to enable email ingestion...      │ │
│  │                                                                  │ │
│  │ [IMAP form continues below...]                                  │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
```

## Table Columns

### 1. Account Column
```
┌──────────────────┐
│ work_email       │ ← Identifier (bold)
│ john@company.com │ ← Username/email (muted, smaller)
└──────────────────┘
```

### 2. Type Column
```
┌──────┐
│ 📧   │ ← Icon
│ IMAP │ ← Badge
└──────┘
```

**Badge Colors:**
- IMAP → Blue (badge-primary)
- SMTP → Purple (badge-secondary)
- OAuth → Pink (badge-accent)
- API Key → Cyan (badge-info)
- Database → Yellow (badge-warning)
- Custom → Gray (badge-ghost)

### 3. Server Column
```
┌──────────────────┐
│ imap.gmail.com   │ ← Hostname
│ Port: 993        │ ← Port (muted, smaller)
└──────────────────┘
```

### 4. Status Column
```
┌─────────┐
│ ✓ Active│ ← Green badge with checkmark
└─────────┘

or

┌──────────┐
│ ✗ Inactive│ ← Gray badge with X
└──────────┘
```

### 5. Actions Column
```
┌────────┐
│ 👁  🗑  │ ← View and Delete buttons (icon buttons)
└────────┘
```

- **👁 (Eye)**: View details (placeholder, not implemented yet)
- **🗑 (Trash)**: Delete credential (with confirmation)

## Empty State

When no credentials exist:

```
┌─────────────────────────────────────────────────────────────────────┐
│ Your Email Accounts                                                 │
│ Manage your configured email accounts                               │
│                                                                     │
│                                                                     │
│                          📧                                         │
│                    (large email icon)                               │
│                                                                     │
│              No email accounts configured yet.                      │
│               Add your first account below.                         │
│                                                                     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Loading State

While fetching credentials:

```
┌─────────────────────────────────────────────────────────────────────┐
│ Your Email Accounts                                                 │
│ Manage your configured email accounts                               │
│                                                                     │
│                                                                     │
│                          🔄                                         │
│                     (large spinner)                                 │
│                                                                     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Delete Confirmation Dialog

```
┌─────────────────────────────────────────────────┐
│  Confirm                                        │
│  ─────────────────────────────────────────────  │
│                                                 │
│  Are you sure you want to delete "work_email"? │
│                                                 │
│                                                 │
│                     [ Cancel ]  [ OK ]          │
└─────────────────────────────────────────────────┘
```

## Success Message (After Delete)

```
┌─────────────────────────────────────────────────────────────────────┐
│ ┌──────────────────────────────────────────────────────────────┐   │
│ │ ✓ Credential "work_email" deleted successfully!              │   │
│ └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## Responsive Behavior

### Desktop View
- Full table visible
- All columns display properly
- Zebra striping for readability
- Hover effects on rows and buttons

### Mobile/Tablet View
- Horizontal scroll enabled (overflow-x-auto)
- Table maintains structure
- Touch-friendly action buttons
- Consistent spacing

## Table Features

### Zebra Striping
```
Row 1: White background (base-100)
Row 2: Gray background (base-200)
Row 3: White background (base-100)
Row 4: Gray background (base-200)
```

### Hover Effects
- Row highlights on hover
- Button backgrounds change on hover
- Delete button turns red background on hover

### Button States
- **View button**: Ghost style, neutral color
- **Delete button**: Ghost style, red text
- **Delete button hover**: Red background, white text

## Data Display Logic

### Server Info
- Shows `service_name` if available
- Shows port below hostname (if available)
- Both fields optional (may be empty for non-IMAP types)

### Status
- Determined by `is_active` field from API
- Active = true → Green badge + checkmark
- Active = false → Gray badge + X icon

### Type Badge
- Extracted from `credential_type` field
- Mapped to appropriate label and color
- Includes icon for visual identification

## Integration with Add Form

When a new credential is added:
1. Form submits successfully
2. Success message shows
3. Form resets to empty state
4. Credentials list automatically refreshes
5. New credential appears at top (most recent first)

## API Data Structure

**List Response:**
```typescript
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

**Table Row Mapping:**
```
Account:  identifier + username
Type:     credential_type → badge
Server:   service_name + port
Status:   is_active → badge
Actions:  view + delete buttons
```

## Color Palette

**Type Badges:**
- IMAP: `badge-primary` (blue: #3B82F6)
- SMTP: `badge-secondary` (purple: #A855F7)
- OAuth: `badge-accent` (pink: #EC4899)
- API Key: `badge-info` (cyan: #06B6D4)
- Database: `badge-warning` (yellow: #F59E0B)
- Custom: `badge-ghost` (gray: transparent with border)

**Status Badges:**
- Active: `badge-success` (green: #10B981)
- Inactive: `badge-ghost` (gray)

**Action Buttons:**
- Default: Neutral gray
- Delete: Error red (#EF4444)
- Delete hover: Red background with white text

## Accessibility

- **Semantic HTML**: Uses `<table>`, `<thead>`, `<tbody>`, `<tr>`, `<td>`
- **Button labels**: Title attributes for icon-only buttons
- **Screen reader friendly**: Proper table structure
- **Keyboard navigation**: Tab through action buttons
- **Clear visual hierarchy**: Bold identifiers, muted secondary text
- **Confirmation dialogs**: Prevent accidental deletions

## Performance

- **Lazy loading**: Only fetches on mount
- **Efficient updates**: Re-fetches entire list (simple approach for now)
- **No pagination**: Suitable for typical number of email accounts (5-20)
- **Future**: Add pagination if list grows large (>50 items)

## Future Enhancements Visual Mockups

### Detail View Modal (Clicking Eye Icon)
```
┌─────────────────────────────────────────────────────────────┐
│  Credential Details                               [ ✕ ]     │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Account Name:        work_email                            │
│  Email Address:       john@company.com                      │
│  Type:                IMAP                                  │
│  Server:              imap.gmail.com                        │
│  Port:                993                                   │
│  TLS/SSL:             ✓ Enabled                             │
│  Auth Method:         Plain                                 │
│  Default Mailbox:     INBOX                                 │
│  Connection Timeout:  30 seconds                            │
│  Validate Certs:      ✓ Enabled                             │
│  Status:              ✓ Active                              │
│  Created:             2026-01-28 10:30 AM                   │
│  Last Accessed:       2026-01-28 11:45 AM                   │
│  Notes:               Work email account                    │
│                                                             │
│                           [ Close ]  [ Edit ]               │
└─────────────────────────────────────────────────────────────┘
```

### Filter/Search Bar (Above Table)
```
┌────────────────────────────────────────────────────────────────┐
│  🔍 [Search accounts...____________]  [Type: All ▼]  [Clear]  │
└────────────────────────────────────────────────────────────────┘
```
