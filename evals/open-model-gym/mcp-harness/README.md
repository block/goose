# MCP Harness

A simulated MCP server with realistic fake tools for testing. Provides mock implementations of common business integrations without requiring actual API credentials.

## Tools Included (35 tools)

### Google Drive
- `gdrive_search` - Search files by name, content, or type
- `gdrive_read_file` - Read file contents
- `gdrive_create_file` - Create new files
- `gdrive_share_file` - Share files with users

### Google Sheets
- `sheets_read` - Read spreadsheet data
- `sheets_write` - Write/update cells
- `sheets_append` - Append rows
- `sheets_create` - Create new spreadsheets

### Salesforce
- `salesforce_query` - Execute SOQL queries
- `salesforce_get_record` - Get record by ID
- `salesforce_create_record` - Create records
- `salesforce_update_record` - Update records
- `salesforce_search` - SOSL search

### Slack
- `slack_send_message` - Send messages
- `slack_get_messages` - Get channel messages
- `slack_search_messages` - Search messages
- `slack_list_channels` - List channels
- `slack_get_user_info` - Get user info
- `slack_set_status` - Set user status

### Google Calendar
- `calendar_list_events` - List events
- `calendar_create_event` - Create events
- `calendar_update_event` - Update events
- `calendar_delete_event` - Delete events

### Gmail
- `gmail_search` - Search emails
- `gmail_read_message` - Read email content
- `gmail_send` - Send emails
- `gmail_create_draft` - Create drafts

### Jira
- `jira_search_issues` - Search with JQL
- `jira_get_issue` - Get issue details
- `jira_create_issue` - Create issues
- `jira_update_issue` - Update issues
- `jira_add_comment` - Add comments

### GitHub
- `github_search_repos` - Search repositories
- `github_list_issues` - List issues
- `github_create_issue` - Create issues
- `github_list_prs` - List pull requests

## Setup

```bash
npm install
npm run build
```

## Run

```bash
npm run start
# or
./run.sh
```

## MCP Config

Add to your MCP client config:

```json
{
  "mcpServers": {
    "harness": {
      "command": "node",
      "args": ["/path/to/mcp-harness/dist/index.js"]
    }
  }
}
```
