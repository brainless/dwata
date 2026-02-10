# Dwata

> Your Personal Data Intelligence Platform

Dwata helps you unlock valuable insights from your own personal data. Whether it's emails, documents, cloud storage, or backups from platforms like LinkedIn and Slack, Dwata brings your information together to give you a clear, historical view of your financial health, business activities, and personal goals.

## What is Dwata?

Dwata is your personal data assistant that transforms raw information from your various sources into actionable insights. Instead of manually tracking through thousands of emails and documents, Dwata does the heavy lifting for you - automatically extracting, organizing, and presenting the information that matters most to you.

**All your data stays local and private.** Dwata runs on your machine, ensuring complete data sovereignty and privacy.

We support bring-your-own Google OAuth apps for Gmail/IMAP access. Set your own `client_id`/`client_secret` in the local config and those will be used.

## What Can Dwata Do?

Dwata includes multiple extractors designed to help you understand different aspects of your personal data:

- **Financial Information** - Track income, expenses, bills, and payments
- **Events & Calendar** - Identify important dates and commitments
- **Business & Work** - Monitor companies, projects, and professional activities
- **More extractors coming soon** - Tasks, contacts, and personal goals

## Financial Insights (Our MVP Feature)

Our financial extraction is the most robust feature currently available in Dwata. It's designed to help you get a complete picture of your financial health from your email history.

### How Financial Extraction Works

#### 1. Smart Email Scanning
Dwata scans your emails using a combination of keywords and patterns to identify financial communications. It looks for terms like "payment," "invoice," "receipt," "transaction," and many others. The scan is lightning-fast and groups emails by sender, showing you which companies or services you interact with most frequently.

#### 2. AI-Powered Pattern Learning
Here's where Dwata gets smart. Instead of using AI to read every email forever (which would be slow and expensive), Dwata uses AI once to learn the pattern:

- You select an email sender from the scan results
- Dwata's AI agent examines sample emails from that sender
- The AI extracts a regex pattern that identifies the financial information (amounts, dates, vendors, etc.)
- You can test and verify the pattern works correctly
- Once saved, the pattern runs directly in the software - no more AI needed!

This approach makes Dwata **incredibly efficient** - you only use AI once per sender, then pattern-based extraction runs instantly.

#### 3. Your Financial Overview
Once patterns are in place, Dwata automatically extracts:
- Income and expenses
- Pending and overdue bills
- Payment history by vendor
- Spending by category
- Net balance and cash flow

<!-- Screenshots will be added here -->

### Supported AI Models

At launch, Dwata supports **Google Gemini models** for the AI-powered pattern extraction. The pattern learning happens quickly and you only need to do it once per email sender.

## What's Coming Next

While we're launching with financial insights as our focus, we're actively working on:

### Data Sources
- **Email** (currently supported via IMAP)
- **Files & Cloud Storage** - Google Drive, Dropbox, OneDrive
- **LinkedIn Backups** - Professional network history
- **Slack Backups** - Team communications and project context
- **And more...**

### New Features
- **Calendar & Events** - Track meetings, deadlines, and important dates
- **Projects & Tasks** - Understand your work and personal projects
- **Business Insights** - See which companies and people you interact with
- **Goal Tracking** - Monitor progress toward personal objectives

Our vision is to give you a complete historical view of your personal and professional life, all extracted from your own data sources.

## Key Benefits

### Privacy First
- All data processing happens locally on your machine
- No data is sent to cloud services except AI pattern extraction (one-time per sender)
- Your information never leaves your control

### Efficiency
- One-time AI pattern learning per source
- After that, extraction is instant and cost-free
- No ongoing AI costs for pattern-based extraction

### Comprehensive
- Multiple data sources in one place
- Historical view of your activities
- Actionable insights from your own data

### Simple to Use
- Clean, intuitive interface
- Focus on what matters to you
- No technical knowledge required

## Use Cases

**Financial Health Monitoring**
Track your spending patterns, identify subscription costs, monitor income streams, and get alerts for upcoming bills.

**Business Development**
Extract leads and opportunities from your communications, understand your network, track project activities.

**Work Overview**
See all your professional activities in one place, understand time allocation, track deliverables and commitments.

**Personal Goals**
Monitor progress toward your objectives using data from your actual activities and communications.

## Getting Started

<!-- Installation instructions will be added here -->

## System Requirements

- macOS, Linux, or Windows
- Internet connection for AI pattern extraction
- Local storage for database

## Support & Community

- **Issues & Bugs**: [GitHub Issues](https://github.com/brainless/dwata/issues)
- **Feature Requests**: [GitHub Discussions](https://github.com/brainless/dwata/discussions)
- **Documentation**: [docs/](./docs/)

## Philosophy

We believe your personal data is your most valuable asset. Rather than letting it sit scattered across dozens of services and accounts, Dwata helps you extract real value from it - while keeping you in complete control.

Dwata doesn't create new data or force you into rigid templates. Instead, it learns from your actual communications and documents, adapting to how you already work and live.

## Technology

Built with modern, efficient technologies:
- **Backend**: Rust (fast, safe, efficient)
- **Database**: SQLite (local, portable, reliable)
- **Frontend**: SolidJS (reactive, performant)
- **AI**: Google Gemini (for pattern learning)

## Contributing

We welcome contributions! Whether it's:
- New data source integrations
- Additional extractors
- UI improvements
- Documentation
- Bug reports and fixes

Check out our [Contributing Guide](DEVELOP.md) to get started.

## License

GPL v3 License - See LICENSE file for details

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

---

**Made for people who want to take control of their personal data and extract meaningful insights from their digital lives.**
