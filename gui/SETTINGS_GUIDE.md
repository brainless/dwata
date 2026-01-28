# Settings Page Guide

## Overview

The Settings page now uses URL-based routing with tabs for better navigation and deep-linking support.

## URL Routes

- `/settings` - General settings (default)
- `/settings/api-keys` - API key configuration
- `/settings/accounts` - Email account management

## File Structure

```
gui/src/pages/
├── Settings.tsx                 # Main settings page with tab navigation
└── settings/
    ├── General.tsx              # General settings tab
    ├── ApiKeys.tsx              # API keys configuration
    └── Accounts.tsx             # Email accounts (IMAP) management
```

## Components

### Settings.tsx
Main container that:
- Renders tab navigation using `<A>` components from SolidJS Router
- Determines active tab from URL pathname
- Conditionally renders tab content using `<Show>` components

### SettingsGeneral (settings/General.tsx)
- Placeholder for general application settings
- Future: app preferences, UI settings, etc.

### SettingsApiKeys (settings/ApiKeys.tsx)
- Manages API keys for external services (e.g., Google Gemini)
- Stores keys in backend configuration file
- Shows configuration status for each key

### SettingsAccounts (settings/Accounts.tsx)
- **Credentials List Table:**
  - Displays existing email accounts
  - Shows identifier, username, type, server, and status
  - Delete functionality with confirmation
  - Empty state when no accounts configured
  - Loading state while fetching
  - Auto-refreshes after add/delete operations

- **Add New Account Form:**
  - Account identification (unique name)
  - Email credentials (username/password)
  - Server settings (host, port, TLS)
  - Authentication method (Plain, OAuth2, XOAUTH2)
  - Advanced settings (mailbox, timeout, certificate validation)
  - Optional notes

- Credentials stored securely using OS keychain via API
- Uses DaisyUI table layout (zebra-striped)
- Similar UI pattern to Tasks page

## IMAP Account Form

### Required Fields
- Account Name: Unique identifier (e.g., "work_email")
- Email Address: Full email address
- Password: Account password (stored in OS keychain)
- IMAP Server Host: Server hostname (e.g., "imap.gmail.com")
- Port: Server port (typically 993 for SSL)

### Optional/Advanced Fields
- Use TLS/SSL: Enable secure connection (default: true)
- Validate SSL Certificates: Verify server certificates (default: true)
- Authentication Method: Plain/OAuth2/XOAUTH2 (default: Plain)
- Default Mailbox: Mailbox to monitor (default: "INBOX")
- Connection Timeout: Seconds before timeout (default: 30)
- Notes: Additional information about the account

### API Integration

The form submits to the credential storage API:

**Endpoint**: `POST http://localhost:8080/api/credentials`

**Request Format**:
```json
{
  "credential_type": "imap",
  "identifier": "work_email",
  "username": "user@example.com",
  "password": "secure_password",
  "service_name": "imap.gmail.com",
  "port": 993,
  "use_tls": true,
  "notes": "Work email account",
  "extra_metadata": "{\"auth_method\":\"plain\",\"default_mailbox\":\"INBOX\",...}"
}
```

**Storage**:
- Password: Stored in OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Metadata: Stored in DuckDB database
- Settings: Serialized as JSON in `extra_metadata` field

**Security**:
- Passwords never stored in database
- Credentials isolated per credential type
- OS-level encryption and access control

## UI Components Used (DaisyUI)

- `card` / `card-body`: Container for settings sections
- `tabs` / `tab` / `tab-active`: Tab navigation
- `form-control`: Form field wrapper
- `label` / `label-text` / `label-text-alt`: Field labels and hints
- `input` / `input-bordered`: Text inputs
- `select` / `select-bordered`: Dropdown selects
- `checkbox` / `checkbox-primary`: Checkboxes for boolean settings
- `textarea` / `textarea-bordered`: Multi-line text input
- `btn` / `btn-primary` / `btn-wide`: Action buttons
- `alert` / `alert-success` / `alert-error`: Status messages
- `loading` / `loading-spinner`: Loading indicators
- `fieldset` / `legend`: Form grouping
- `divider`: Visual section separator

## Adding New Settings Tabs

1. Create new component in `src/pages/settings/YourTab.tsx`
2. Add route handler in `Settings.tsx`:
   ```tsx
   <A href="/settings/your-tab" class={`tab ${activeTab() === "your-tab" ? "tab-active" : ""}`}>
     Your Tab
   </A>
   ```
3. Add conditional render:
   ```tsx
   <Show when={activeTab() === "your-tab"}>
     <YourTab />
   </Show>
   ```
4. Update `activeTab()` function to handle new route

## Common Patterns

### Form State Management
```tsx
const [fieldName, setFieldName] = createSignal("default");
```

### API Calls
```tsx
const response = await fetch("http://localhost:8080/api/endpoint", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(data),
});
```

### Loading States
```tsx
const [isLoading, setIsLoading] = createSignal(false);

// In button
disabled={isLoading()}
{isLoading() ? "Loading..." : "Submit"}
```

### Success/Error Messages
```tsx
const [message, setMessage] = createSignal("");
const [messageType, setMessageType] = createSignal<"success" | "error">("success");

// Display
{message() && (
  <div class={`alert ${messageType() === "success" ? "alert-success" : "alert-error"}`}>
    <span>{message()}</span>
  </div>
)}
```

## Future Enhancements

- List existing IMAP accounts with edit/delete actions
- Test connection button to verify IMAP settings
- SMTP account configuration
- OAuth2 flow for Gmail/Outlook
- Credential management (view, edit, delete)
- Account sync status indicators
- Bulk import/export of accounts
