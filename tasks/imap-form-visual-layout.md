# IMAP Form Visual Layout

## Page Structure

```
┌─────────────────────────────────────────────────────────────────────┐
│  Settings                                                           │
│                                                                     │
│  [General] [API Keys] [Accounts]  ← URL-based tabs                 │
│  ─────────────────────────────────────────────────────────────────  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ Email Accounts                                                │  │
│  │ Add your IMAP email accounts to enable email ingestion and   │  │
│  │ monitoring.                                                   │  │
│  │                                                               │  │
│  │ ┌──────────────────────────────────────────────────────────┐ │  │
│  │ │ IMAP Email Account                                        │ │  │
│  │ ├──────────────────────────────────────────────────────────┤ │  │
│  │ │                                                           │ │  │
│  │ │  Account Name *           [Unique identifier]            │ │  │
│  │ │  [work_email, personal_gmail________________]            │ │  │
│  │ │                                                           │ │  │
│  │ │  Email Address *                                         │ │  │
│  │ │  [your.email@example.com___________________]            │ │  │
│  │ │                                                           │ │  │
│  │ │  Password *               [Stored securely in keychain]  │ │  │
│  │ │  [••••••••••••••••••••••••••••••••••••••••]            │ │  │
│  │ │                                                           │ │  │
│  │ │  ─────────── Server Settings ───────────                 │ │  │
│  │ │                                                           │ │  │
│  │ │  IMAP Server Host *           Port *                     │ │  │
│  │ │  [imap.gmail.com______]       [993]                      │ │  │
│  │ │                                                           │ │  │
│  │ │  ☑ Use TLS/SSL                ☑ Validate SSL Certs      │ │  │
│  │ │    Recommended for secure       Should be enabled in     │ │  │
│  │ │    connection                   production               │ │  │
│  │ │                                                           │ │  │
│  │ │  ───────── Advanced Settings ──────────                  │ │  │
│  │ │                                                           │ │  │
│  │ │  Auth Method    Default Mailbox    Timeout (sec)         │ │  │
│  │ │  [Plain    ▼]   [INBOX________]    [30]                 │ │  │
│  │ │                                                           │ │  │
│  │ │  Notes (Optional)                                        │ │  │
│  │ │  [Additional notes about this account...              ] │ │  │
│  │ │  [                                                     ] │ │  │
│  │ │                                                           │ │  │
│  │ └──────────────────────────────────────────────────────────┘ │  │
│  │                                                               │  │
│  │  ┌──────────────────────────────────────────────────────┐   │  │
│  │  │ ✓ IMAP account added successfully!                   │   │  │
│  │  └──────────────────────────────────────────────────────┘   │  │
│  │                                                               │  │
│  │                               [  Add IMAP Account  ]  ←CTA   │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Component Breakdown

### Tab Navigation
```
[General] [API Keys] [Accounts]
  │          │           └─── Active (underlined)
  └─ /settings
             └─ /settings/api-keys
                            └─ /settings/accounts
```

### Form Sections

#### 1. Account Information
- **Account Name** (text input, required)
  - Placeholder: "e.g., work_email, personal_gmail"
  - Helper: "Unique identifier"

- **Email Address** (email input, required)
  - Placeholder: "your.email@example.com"

- **Password** (password input, required)
  - Placeholder: "••••••••"
  - Helper: "Stored securely in OS keychain"

#### 2. Server Settings
- **IMAP Server Host** (text input, required)
  - Placeholder: "imap.gmail.com"
  - Half-width on desktop

- **Port** (number input, required)
  - Placeholder: "993"
  - Half-width on desktop

- **Use TLS/SSL** (checkbox, default: checked)
  - Label: "Use TLS/SSL"
  - Description: "Recommended for secure connection"

- **Validate SSL Certificates** (checkbox, default: checked)
  - Label: "Validate SSL Certificates"
  - Description: "Should be enabled in production"

#### 3. Advanced Settings
- **Authentication Method** (select dropdown)
  - Options: Plain, OAuth2, XOAUTH2
  - Default: Plain
  - Third-width on desktop

- **Default Mailbox** (text input)
  - Placeholder: "INBOX"
  - Default: "INBOX"
  - Third-width on desktop

- **Connection Timeout** (number input)
  - Placeholder: "30"
  - Default: 30
  - Label: "Connection Timeout (sec)"
  - Third-width on desktop

#### 4. Notes
- **Notes** (textarea, optional)
  - Placeholder: "Additional notes about this account..."
  - Rows: 2
  - Full-width

### Action Button
- **Add IMAP Account** (primary button, wide)
  - Position: Bottom right
  - States:
    - Default: "Add IMAP Account"
    - Loading: "🔄 Adding Account..." (with spinner)
    - Disabled: When loading

### Feedback Messages
- **Success Alert** (green)
  - "IMAP account added successfully!"

- **Error Alert** (red)
  - Shows API error message or generic failure message

## Responsive Behavior

### Desktop (lg and up)
- Two-column layout for host/port
- Three-column layout for advanced settings
- Wide form (card auto-sizes)

### Mobile/Tablet
- Single column layout
- Full-width inputs
- Stacked checkboxes
- Stacked advanced settings

## Color Scheme (DaisyUI)

- **Primary color**: Buttons, checkboxes, active tabs
- **Base-100**: Card background (white/light in light mode)
- **Base-200**: Page background (slightly darker than card)
- **Base-300**: Tab border
- **Success**: Green alert for success messages
- **Error**: Red alert for error messages

## State Indicators

### Loading State
```
[🔄 Adding Account...]  (disabled, with spinner)
```

### Success State
```
┌─────────────────────────────────────────┐
│ ✓ IMAP account added successfully!     │
└─────────────────────────────────────────┘

[  Add IMAP Account  ]  (enabled, form cleared)
```

### Error State
```
┌─────────────────────────────────────────┐
│ ✗ Failed to add IMAP account.          │
│   Error: Duplicate identifier           │
└─────────────────────────────────────────┘

[  Add IMAP Account  ]  (enabled, form retained)
```

## Validation

### Client-side (HTML5)
- Required fields marked with `required` attribute
- Email field uses `type="email"`
- Number fields use `type="number"`
- Form won't submit if required fields empty

### Server-side
- API validates duplicate identifiers
- Returns detailed error messages
- Displayed in error alert

## Accessibility Features

- All inputs have associated labels
- Helper text provides context
- Required fields marked with asterisk (*)
- Semantic HTML (fieldset, legend)
- Focus states on all interactive elements
- Error messages clearly displayed

## Example Gmail Configuration

```
Account Name:        personal_gmail
Email Address:       yourname@gmail.com
Password:            your-app-password
IMAP Server Host:    imap.gmail.com
Port:                993
Use TLS/SSL:         ✓
Validate SSL Certs:  ✓
Auth Method:         Plain
Default Mailbox:     INBOX
Timeout:             30
Notes:               Personal Gmail account
```

## Example Outlook Configuration

```
Account Name:        work_outlook
Email Address:       yourname@outlook.com
Password:            your-password
IMAP Server Host:    outlook.office365.com
Port:                993
Use TLS/SSL:         ✓
Validate SSL Certs:  ✓
Auth Method:         Plain
Default Mailbox:     INBOX
Timeout:             30
Notes:               Work Outlook account
```
