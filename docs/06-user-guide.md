# Dwata User Guide

## Getting Started

### Installation

1. **Download Dwata**:
   ```bash
   # Clone the repository
   git clone https://github.com/brainless/dwata.git
   cd dwata
   ```

2. **Install Dependencies**:
   ```bash
   # Build the project
   cargo build --release
   ```

3. **Configure LLM Provider**:
   - Set up your API key for your preferred LLM provider
   - Currently supported: Anthropic Claude, OpenAI
   - Configuration via environment variables or config file

4. **Start the Server**:
   ```bash
   # Run the API server
   cargo run --release
   ```

5. **Access GUI**:
   - Open your browser to `http://localhost:3000` (or configured port)
   - The GUI provides an intuitive interface for all dwata features

## Core Concepts

### Projects

A **project** is a collection of related data organized around a specific domain:
- **Finance Project**: Accounts, transactions, budgets
- **Travel Project**: Destinations, itineraries, bookings
- **Work Project**: Tasks, goals, timelines

Each project has:
- Dynamic database tables
- Associated requirements and context
- Historical conversations with agents

### Agents

**Agents** are AI assistants specialized for specific tasks:
- **Requirements Gatherers**: Ask questions and structure your data
- **Analysts**: Query and analyze your information
- **Planners**: Help organize and forecast

Agents work together:
1. First, a requirements agent sets up your project
2. Then, analysis agents help you explore your data
3. Planning agents assist with future actions

### Sessions

A **session** is a conversation with an agent:
- Each session has a clear objective
- Sessions are saved automatically
- You can resume sessions later
- All tool calls and results are tracked

## Common Use Cases

### 1. Personal Finance Tracking

**Goal**: Understand your financial situation and spending habits

**Steps**:

1. **Create Finance Project**:
   ```
   User: "I want to track my personal finances"
   ```

2. **Requirements Gathering**:
   ```
   Agent: "I'll help you set up finance tracking. Let me ask:
           1. What banks do you use?
           2. Do you have credit cards?
           3. What are your main income sources?
           4. How do you categorize expenses?"

   User: "I use Chase and Bank of America. I have two credit cards.
          I have a salary and some freelance income. I track groceries,
          dining, utilities, entertainment, and transportation."
   ```

3. **Data Entry** (multiple options):
   - Manual entry through GUI
   - CSV import from bank exports
   - OCR from receipts using Tesseract agent
   - API connections to financial services

4. **Ask Questions**:
   ```
   "What's my spending this month?"
   "How much did I spend on dining in the last quarter?"
   "What's my net worth?"
   "Can you forecast my savings for next year?"
   ```

5. **Get Insights**:
   - Spending breakdowns by category
   - Income vs. expense trends
   - Budget recommendations
   - Financial health score

### 2. Travel Planning

**Goal**: Plan and organize a trip

**Steps**:

1. **Start Planning**:
   ```
   User: "I want to plan a trip to Japan"
   ```

2. **Gather Requirements**:
   ```
   Agent: "Great! Let me understand your travel plans:
           1. When are you planning to go?
           2. How long is your trip?
           3. What's your budget?
           4. What interests you? (culture, food, nature, cities)"

   User: "Going in April for 2 weeks. Budget is $5000.
          Interested in culture, food, and traditional sites."
   ```

3. **Research Phase**:
   - Agent searches for destinations
   - Suggests itinerary options
   - Estimates costs
   - Identifies must-see locations

4. **Create Itinerary**:
   - Day-by-day schedule
   - Transportation between cities
   - Accommodation suggestions
   - Restaurant recommendations

5. **Track Bookings**:
   - Record reservations
   - Store confirmation numbers
   - Track spending vs. budget
   - Countdown to trip

6. **During Trip**:
   ```
   "What's nearby that's interesting?"
   "Where should we eat tonight?"
   "How much have we spent so far?"
   ```

### 3. Work and Goal Management

**Goal**: Track professional tasks and personal objectives

**Steps**:

1. **Set Up Projects**:
   ```
   User: "I want to track my work projects and personal goals"
   ```

2. **Define Goals**:
   ```
   Agent: "Let's organize your goals:
           1. What work projects are you involved in?
           2. What are your career objectives?
           3. What personal goals do you have?
           4. What timeframe are you working with?"
   ```

3. **Break Down Tasks**:
   - Projects divided into tasks
   - Tasks have deadlines and priorities
   - Dependencies tracked
   - Progress monitored

4. **Regular Check-ins**:
   ```
   "What tasks are due this week?"
   "How am I progressing on Project X?"
   "What goals should I focus on?"
   "Show me my completed milestones"
   ```

5. **Analytics**:
   - Completion rates
   - Time spent per project
   - Goal achievement trends
   - Productivity insights

## Working with Agents

### Starting a Conversation

1. **Choose Your Intent**:
   - Starting a new project? Use requirements gathering
   - Asking about existing data? Use analysis agent
   - Need clarification? Agent will determine automatically

2. **Be Specific**:
   ```
   ❌ "Tell me about money"
   ✅ "What were my top 5 expense categories last month?"

   ❌ "Travel stuff"
   ✅ "Plan a 10-day trip to Italy in June with a $4000 budget"
   ```

3. **Provide Context**:
   - Reference specific time periods
   - Mention relevant details upfront
   - Include constraints or preferences

### Answering Agent Questions

**Do**:
- Answer completely
- Provide specific numbers when possible
- Mention exceptions or special cases
- Ask for clarification if question is unclear

**Don't**:
- Give one-word answers when detail helps
- Skip questions (say "skip" if not relevant)
- Provide contradictory information

**Example**:
```
Agent: "What are your monthly expenses?"

❌ "Some"
❌ "Around $3000"
✅ "Rent: $1500, Groceries: $400, Utilities: $150,
    Transportation: $200, Entertainment: $300,
    Other: $200, Total: ~$2750"
```

### Refining Requirements

Requirements can evolve:

```
User: "I want to add credit card tracking to my finance project"

Agent: "I'll extend your finance project:
        1. Which credit cards?
        2. Should I track payment due dates?
        3. Do you want rewards tracking?"
```

The agent will:
- Update database schema
- Preserve existing data
- Add new tables/columns as needed

### Understanding Results

Agents provide:
- **Direct Answers**: Simple queries get simple answers
- **Detailed Analysis**: Complex questions get breakdowns
- **Visualizations**: GUI displays charts and graphs
- **Action Items**: Suggestions for next steps

## Data Management

### Importing Data

**CSV Import**:
```
Settings > Import Data > Select CSV
- Map columns to fields
- Preview before importing
- Batch import supported
```

**Manual Entry**:
```
Project > Add Transaction/Entry
- Form-based input
- Auto-complete for categories
- Quick entry mode available
```

**OCR from Images**:
```
Upload Receipt > Extract Data
- Tesseract agent reads text
- Parses into structured data
- Review and confirm
```

**API Connections**:
```
Settings > Connect Service
- Authenticate with provider
- Configure sync frequency
- Map data fields
```

### Exporting Data

Export your data anytime:

```
Project > Export
- Format: CSV, JSON, Excel
- Date range selection
- Choose tables/fields
```

Your SQLite database can also be accessed directly:
```bash
sqlite3 ~/.dwata/dwata.db
```

### Privacy and Security

- **Local Storage**: All data stays on your machine
- **No Cloud Sync**: Unless you explicitly enable it
- **Encrypted Database**: Optional encryption support
- **API Key Security**: Keys stored securely in OS keychain
- **Data Ownership**: You own all your data

## Tips and Tricks

### Efficient Conversations

1. **Bundle Questions**:
   ```
   ✅ "What's my total spending this month, how does it compare
       to last month, and what's my biggest expense category?"

   vs.

   ❌ Three separate questions one at a time
   ```

2. **Use Context**:
   ```
   "In my finance project, show me trends over the last 6 months"
   ```

3. **Reference Previous Sessions**:
   ```
   "Remember when we discussed my savings goal? How am I tracking?"
   ```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl/Cmd + N` | New conversation |
| `Ctrl/Cmd + K` | Quick command |
| `Ctrl/Cmd + P` | Switch project |
| `Ctrl/Cmd + F` | Search history |
| `Esc` | Cancel current action |

### Power User Features

**Batch Operations**:
```
"Import all receipts from my downloads folder from the last month"
```

**Scheduled Queries**:
```
"Send me a weekly summary of my spending every Monday"
```

**Custom Alerts**:
```
"Alert me when my checking account goes below $1000"
```

**Cross-Project Queries**:
```
"Show me all travel expenses from my finance project for trips
 in my travel project"
```

## Troubleshooting

### Agent Not Responding

1. Check server logs
2. Verify LLM API key is valid
3. Check internet connection
4. Restart dwata-api if needed

### Data Not Saving

1. Check database file permissions
2. Verify disk space available
3. Check for database locks
4. Review error messages in GUI

### Incorrect Results

1. Rephrase your question more specifically
2. Verify data was entered correctly
3. Check date ranges in query
4. Ask agent to "explain your reasoning"

### Performance Issues

1. Archive old sessions
2. Optimize database (vacuum)
3. Reduce concurrent sessions
4. Check system resources

## Getting Help

### In-App Help

- **Hover tooltips**: Hover over UI elements
- **Help button**: Context-sensitive help
- **Example prompts**: Suggested questions in each project

### Documentation

- **Overview**: [01-overview.md](./01-overview.md)
- **Architecture**: [02-architecture.md](./02-architecture.md)
- **Database**: [03-database-schema.md](./03-database-schema.md)
- **Agents**: [04-agents-reference.md](./04-agents-reference.md)
- **API**: [05-api-reference.md](./05-api-reference.md)

### Community

- GitHub Issues: Report bugs or request features
- Discussions: Share use cases and tips
- Discord: Real-time community support (coming soon)

## Advanced Features

### Custom Agents

Create your own specialized agents:

```rust
// See developer documentation for details
pub struct CustomAgent { ... }
```

### Database Queries

Direct SQL access for power users:

```
Query Mode > SQL
SELECT * FROM finance_transactions
WHERE amount > 100 AND date > '2024-01-01'
```

### API Integration

Build custom integrations:

```typescript
import { DwataClient } from 'dwata-api';

const client = new DwataClient('http://localhost:8080');
const session = await client.createSession({
  agent_name: 'requirements_gathering',
  user_prompt: 'Track my workouts',
});
```

### Automation

Create workflows:

```yaml
# .dwata/workflows/weekly-review.yaml
name: Weekly Financial Review
schedule: "0 9 * * 1"  # Every Monday at 9am
steps:
  - query: "Summarize last week's spending"
  - notify: email
```

## Best Practices

1. **Regular Updates**: Keep projects current with recent data
2. **Descriptive Names**: Use clear project and category names
3. **Consistent Categories**: Stick to your categorization scheme
4. **Review Results**: Always verify agent outputs
5. **Backup Data**: Export important projects regularly
6. **Archive Old Data**: Keep database lean and fast
7. **Refine Over Time**: Update requirements as needs change

## Next Steps

- Explore [Agents Reference](./04-agents-reference.md) for detailed agent capabilities
- Check [API Reference](./05-api-reference.md) for integration options
- Review [Database Schema](./03-database-schema.md) for custom queries
- See [Architecture](./02-architecture.md) for system understanding

Happy tracking with Dwata!
