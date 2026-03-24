import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import * as fs from 'fs';
import * as path from 'path';

// Logging configuration
const LOG_FILE = process.env.MCP_HARNESS_LOG || path.join(process.cwd(), 'tool-calls.log');

function logToolCall(toolName: string, args: Record<string, any>, result: any) {
  const entry = {
    timestamp: new Date().toISOString(),
    tool: toolName,
    arguments: args,
    result: result,
  };
  const line = JSON.stringify(entry) + '\n';
  fs.appendFileSync(LOG_FILE, line);
}

// Fake data generators
const fakeUsers = [
  { id: 'U001', name: 'Alice Johnson', email: 'alice@company.com', department: 'Engineering' },
  { id: 'U002', name: 'Bob Smith', email: 'bob@company.com', department: 'Sales' },
  { id: 'U003', name: 'Carol Williams', email: 'carol@company.com', department: 'Marketing' },
  { id: 'U004', name: 'David Brown', email: 'david@company.com', department: 'Finance' },
  { id: 'U005', name: 'Emma Davis', email: 'emma@company.com', department: 'Engineering' },
];

const fakeCompanies = [
  { id: 'ACC001', name: 'Acme Corp', industry: 'Technology', revenue: 5000000 },
  { id: 'ACC002', name: 'GlobalTech Inc', industry: 'Manufacturing', revenue: 12000000 },
  { id: 'ACC003', name: 'StartupXYZ', industry: 'SaaS', revenue: 800000 },
  { id: 'ACC004', name: 'MegaCorp Ltd', industry: 'Retail', revenue: 45000000 },
  { id: 'ACC005', name: 'InnovateCo', industry: 'Healthcare', revenue: 3200000 },
];

const fakeOpportunities = [
  { id: 'OPP001', name: 'Enterprise Deal - Acme', accountId: 'ACC001', stage: 'Negotiation', amount: 150000, closeDate: '2026-03-15' },
  { id: 'OPP002', name: 'Expansion - GlobalTech', accountId: 'ACC002', stage: 'Proposal', amount: 75000, closeDate: '2026-02-28' },
  { id: 'OPP003', name: 'New Business - StartupXYZ', accountId: 'ACC003', stage: 'Discovery', amount: 25000, closeDate: '2026-04-10' },
  { id: 'OPP004', name: 'Renewal - MegaCorp', accountId: 'ACC004', stage: 'Closed Won', amount: 200000, closeDate: '2026-01-20' },
];

const fakeFiles = [
  { id: 'FILE001', name: 'Q4 Report.docx', mimeType: 'application/vnd.google-apps.document', size: 245000, modifiedTime: '2026-01-28T14:30:00Z', owner: 'alice@company.com' },
  { id: 'FILE002', name: 'Sales Forecast.xlsx', mimeType: 'application/vnd.google-apps.spreadsheet', size: 128000, modifiedTime: '2026-02-01T09:15:00Z', owner: 'bob@company.com' },
  { id: 'FILE003', name: 'Marketing Plan 2026.pdf', mimeType: 'application/pdf', size: 1520000, modifiedTime: '2026-01-25T16:45:00Z', owner: 'carol@company.com' },
  { id: 'FILE004', name: 'Budget Template.xlsx', mimeType: 'application/vnd.google-apps.spreadsheet', size: 89000, modifiedTime: '2026-01-30T11:00:00Z', owner: 'david@company.com' },
  { id: 'FILE005', name: 'Architecture Diagram.png', mimeType: 'image/png', size: 456000, modifiedTime: '2026-02-02T08:20:00Z', owner: 'emma@company.com' },
];

const fakeSpreadsheets: Record<string, { title: string; sheets: { name: string; data: string[][] }[] }> = {
  'SHEET001': {
    title: 'Sales Pipeline Q1 2026',
    sheets: [
      {
        name: 'Deals',
        data: [
          ['Deal Name', 'Company', 'Amount', 'Stage', 'Close Date'],
          ['Enterprise License', 'Acme Corp', '$150,000', 'Negotiation', '2026-03-15'],
          ['Platform Upgrade', 'GlobalTech', '$75,000', 'Proposal', '2026-02-28'],
          ['Starter Package', 'StartupXYZ', '$25,000', 'Discovery', '2026-04-10'],
        ]
      },
      {
        name: 'Summary',
        data: [
          ['Metric', 'Value'],
          ['Total Pipeline', '$250,000'],
          ['Deals in Negotiation', '1'],
          ['Expected Close Rate', '65%'],
        ]
      }
    ]
  },
  'SHEET002': {
    title: 'Employee Directory',
    sheets: [
      {
        name: 'Employees',
        data: [
          ['Name', 'Email', 'Department', 'Start Date'],
          ['Alice Johnson', 'alice@company.com', 'Engineering', '2022-03-01'],
          ['Bob Smith', 'bob@company.com', 'Sales', '2021-08-15'],
          ['Carol Williams', 'carol@company.com', 'Marketing', '2023-01-10'],
        ]
      }
    ]
  }
};

const fakeSlackChannels = [
  { id: 'C001', name: 'general', memberCount: 150, topic: 'Company-wide announcements' },
  { id: 'C002', name: 'engineering', memberCount: 45, topic: 'Engineering discussions' },
  { id: 'C003', name: 'sales', memberCount: 28, topic: 'Sales team coordination' },
  { id: 'C004', name: 'random', memberCount: 142, topic: 'Non-work banter' },
];

const fakeSlackMessages = [
  { channel: 'C001', user: 'U001', text: 'Reminder: All-hands meeting tomorrow at 2pm', ts: '1706886000.000100' },
  { channel: 'C001', user: 'U003', text: 'Thanks for the reminder!', ts: '1706886060.000200' },
  { channel: 'C002', user: 'U005', text: 'Just merged the new auth PR', ts: '1706885400.000300' },
  { channel: 'C002', user: 'U001', text: 'Great work! Any breaking changes?', ts: '1706885460.000400' },
  { channel: 'C003', user: 'U002', text: 'Closed the MegaCorp deal! 🎉', ts: '1706884800.000500' },
  { channel: 'C001', user: 'U004', text: 'Please review the quarterly review document I shared. Key metrics show 15% growth.', ts: '1706886100.000600' },
];

const fakeCalendarEvents = [
  { id: 'EVT001', summary: 'Weekly Standup', start: '2026-02-03T09:00:00Z', end: '2026-02-03T09:30:00Z', attendees: ['alice@company.com', 'emma@company.com'] },
  { id: 'EVT002', summary: 'Client Call - Acme Corp', start: '2026-02-03T14:00:00Z', end: '2026-02-03T15:00:00Z', attendees: ['bob@company.com', 'alice@company.com'] },
  { id: 'EVT003', summary: 'Product Review', start: '2026-02-04T11:00:00Z', end: '2026-02-04T12:00:00Z', attendees: ['carol@company.com', 'david@company.com', 'emma@company.com'] },
];

const fakeEmails = [
  { id: 'MSG001', from: 'client@acme.com', to: 'bob@company.com', subject: 'Re: Proposal Follow-up', snippet: 'Thanks for sending over the revised proposal...', date: '2026-02-02T10:30:00Z' },
  { id: 'MSG002', from: 'hr@company.com', to: 'all@company.com', subject: 'February Benefits Update', snippet: 'Please review the updated benefits information...', date: '2026-02-01T08:00:00Z' },
  { id: 'MSG003', from: 'alice@company.com', to: 'emma@company.com', subject: 'Code Review Request', snippet: 'Could you take a look at PR #423...', date: '2026-02-02T14:15:00Z' },
];

// Utility functions
function generateId(prefix: string): string {
  return `${prefix}${Date.now().toString(36)}${Math.random().toString(36).substr(2, 5)}`;
}

function now(): string {
  return new Date().toISOString();
}

function randomDelay(): number {
  return Math.floor(Math.random() * 200) + 50;
}

// Tool definitions
const tools = [
  // === Google Drive Tools ===
  {
    name: 'gdrive_search',
    description: 'Search for files in Google Drive by name, content, or type. Returns matching files with metadata.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search query (supports name:, type:, owner: prefixes)' },
        limit: { type: 'integer', minimum: 1, maximum: 50, default: 10, description: 'Maximum results to return' },
        includeShared: { type: 'boolean', default: true, description: 'Include files shared with you' },
      },
      required: ['query'],
    },
  },
  {
    name: 'gdrive_read_file',
    description: 'Read the contents of a file from Google Drive. Supports documents, text files, and exports spreadsheets as CSV.',
    inputSchema: {
      type: 'object',
      properties: {
        fileId: { type: 'string', description: 'The Google Drive file ID' },
        exportFormat: { type: 'string', enum: ['text', 'html', 'csv', 'pdf'], default: 'text', description: 'Export format for Google Docs' },
      },
      required: ['fileId'],
    },
  },
  {
    name: 'gdrive_create_file',
    description: 'Create a new file in Google Drive with the specified content.',
    inputSchema: {
      type: 'object',
      properties: {
        name: { type: 'string', description: 'File name including extension' },
        content: { type: 'string', description: 'File content' },
        mimeType: { type: 'string', description: 'MIME type of the file' },
        folderId: { type: 'string', description: 'Parent folder ID (optional)' },
      },
      required: ['name', 'content'],
    },
  },
  {
    name: 'gdrive_share_file',
    description: 'Share a file with specific users or make it publicly accessible.',
    inputSchema: {
      type: 'object',
      properties: {
        fileId: { type: 'string', description: 'The Google Drive file ID' },
        email: { type: 'string', description: 'Email address to share with' },
        role: { type: 'string', enum: ['reader', 'commenter', 'writer'], default: 'reader', description: 'Permission level' },
        sendNotification: { type: 'boolean', default: true, description: 'Send email notification' },
      },
      required: ['fileId', 'email'],
    },
  },

  // === Google Sheets Tools ===
  {
    name: 'sheets_read',
    description: 'Read data from a Google Sheets spreadsheet. Returns cell values from the specified range.',
    inputSchema: {
      type: 'object',
      properties: {
        spreadsheetId: { type: 'string', description: 'The spreadsheet ID' },
        range: { type: 'string', description: 'A1 notation range (e.g., "Sheet1!A1:D10")' },
        valueRenderOption: { type: 'string', enum: ['FORMATTED_VALUE', 'UNFORMATTED_VALUE', 'FORMULA'], default: 'FORMATTED_VALUE' },
      },
      required: ['spreadsheetId', 'range'],
    },
  },
  {
    name: 'sheets_write',
    description: 'Write data to a Google Sheets spreadsheet. Overwrites existing data in the specified range.',
    inputSchema: {
      type: 'object',
      properties: {
        spreadsheetId: { type: 'string', description: 'The spreadsheet ID' },
        range: { type: 'string', description: 'A1 notation range (e.g., "Sheet1!A1")' },
        values: { type: 'array', items: { type: 'array', items: { type: 'string' } }, description: '2D array of values to write' },
      },
      required: ['spreadsheetId', 'range', 'values'],
    },
  },
  {
    name: 'sheets_append',
    description: 'Append rows to a Google Sheets spreadsheet. Adds data after the last row with content.',
    inputSchema: {
      type: 'object',
      properties: {
        spreadsheetId: { type: 'string', description: 'The spreadsheet ID' },
        range: { type: 'string', description: 'A1 notation range indicating the table (e.g., "Sheet1!A:D")' },
        values: { type: 'array', items: { type: 'array', items: { type: 'string' } }, description: '2D array of rows to append' },
      },
      required: ['spreadsheetId', 'range', 'values'],
    },
  },
  {
    name: 'sheets_create',
    description: 'Create a new Google Sheets spreadsheet with optional initial data.',
    inputSchema: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Spreadsheet title' },
        sheetNames: { type: 'array', items: { type: 'string' }, description: 'Names of sheets to create' },
        initialData: { type: 'object', description: 'Map of sheet name to 2D array of initial values' },
      },
      required: ['title'],
    },
  },

  // === Salesforce Tools ===
  {
    name: 'salesforce_query',
    description: 'Execute a SOQL query against Salesforce. Returns matching records with pagination support.',
    inputSchema: {
      type: 'object',
      properties: {
        soql: { type: 'string', description: 'SOQL query (e.g., "SELECT Id, Name FROM Account WHERE Industry = \'Technology\'")' },
        limit: { type: 'integer', minimum: 1, maximum: 2000, default: 100, description: 'Maximum records to return' },
      },
      required: ['soql'],
    },
  },
  {
    name: 'salesforce_get_record',
    description: 'Get a single Salesforce record by ID with all or specified fields.',
    inputSchema: {
      type: 'object',
      properties: {
        objectType: { type: 'string', description: 'Salesforce object type (e.g., Account, Contact, Opportunity)' },
        recordId: { type: 'string', description: 'The record ID' },
        fields: { type: 'array', items: { type: 'string' }, description: 'Fields to retrieve (optional, returns all if not specified)' },
      },
      required: ['objectType', 'recordId'],
    },
  },
  {
    name: 'salesforce_create_record',
    description: 'Create a new record in Salesforce.',
    inputSchema: {
      type: 'object',
      properties: {
        objectType: { type: 'string', description: 'Salesforce object type' },
        data: { type: 'object', description: 'Field values for the new record' },
      },
      required: ['objectType', 'data'],
    },
  },
  {
    name: 'salesforce_update_record',
    description: 'Update an existing Salesforce record.',
    inputSchema: {
      type: 'object',
      properties: {
        objectType: { type: 'string', description: 'Salesforce object type' },
        recordId: { type: 'string', description: 'The record ID to update' },
        data: { type: 'object', description: 'Field values to update' },
      },
      required: ['objectType', 'recordId', 'data'],
    },
  },
  {
    name: 'salesforce_search',
    description: 'Execute a SOSL search across multiple Salesforce objects.',
    inputSchema: {
      type: 'object',
      properties: {
        searchTerm: { type: 'string', description: 'Search term' },
        objects: { type: 'array', items: { type: 'string' }, description: 'Objects to search (e.g., ["Account", "Contact"])' },
        limit: { type: 'integer', minimum: 1, maximum: 200, default: 20 },
      },
      required: ['searchTerm'],
    },
  },

  // === Slack Tools ===
  {
    name: 'slack_send_message',
    description: 'Send a message to a Slack channel or direct message.',
    inputSchema: {
      type: 'object',
      properties: {
        channel: { type: 'string', description: 'Channel ID or name (e.g., "#general" or "C001")' },
        text: { type: 'string', description: 'Message text (supports Slack markdown)' },
        threadTs: { type: 'string', description: 'Thread timestamp to reply to (optional)' },
      },
      required: ['channel', 'text'],
    },
  },
  {
    name: 'slack_get_messages',
    description: 'Retrieve recent messages from a Slack channel.',
    inputSchema: {
      type: 'object',
      properties: {
        channel: { type: 'string', description: 'Channel ID or name' },
        limit: { type: 'integer', minimum: 1, maximum: 100, default: 20 },
        oldest: { type: 'string', description: 'Only messages after this timestamp' },
        latest: { type: 'string', description: 'Only messages before this timestamp' },
      },
      required: ['channel'],
    },
  },
  {
    name: 'slack_search_messages',
    description: 'Search for messages across Slack channels.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search query (supports from:, in:, has: modifiers)' },
        sort: { type: 'string', enum: ['score', 'timestamp'], default: 'score' },
        limit: { type: 'integer', minimum: 1, maximum: 100, default: 20 },
      },
      required: ['query'],
    },
  },
  {
    name: 'slack_list_channels',
    description: 'List available Slack channels the user has access to.',
    inputSchema: {
      type: 'object',
      properties: {
        types: { type: 'string', enum: ['public', 'private', 'mpim', 'im', 'all'], default: 'public' },
        limit: { type: 'integer', minimum: 1, maximum: 200, default: 50 },
      },
    },
  },
  {
    name: 'slack_get_user_info',
    description: 'Get information about a Slack user.',
    inputSchema: {
      type: 'object',
      properties: {
        userId: { type: 'string', description: 'User ID' },
      },
      required: ['userId'],
    },
  },
  {
    name: 'slack_set_status',
    description: 'Set your Slack status message and emoji.',
    inputSchema: {
      type: 'object',
      properties: {
        statusText: { type: 'string', description: 'Status message' },
        statusEmoji: { type: 'string', description: 'Status emoji (e.g., ":calendar:")' },
        expirationMinutes: { type: 'integer', description: 'Minutes until status expires (optional)' },
      },
      required: ['statusText'],
    },
  },

  // === Google Calendar Tools ===
  {
    name: 'calendar_list_events',
    description: 'List upcoming calendar events.',
    inputSchema: {
      type: 'object',
      properties: {
        calendarId: { type: 'string', default: 'primary', description: 'Calendar ID' },
        timeMin: { type: 'string', description: 'Start time (ISO 8601)' },
        timeMax: { type: 'string', description: 'End time (ISO 8601)' },
        maxResults: { type: 'integer', minimum: 1, maximum: 250, default: 10 },
      },
    },
  },
  {
    name: 'calendar_create_event',
    description: 'Create a new calendar event.',
    inputSchema: {
      type: 'object',
      properties: {
        summary: { type: 'string', description: 'Event title' },
        description: { type: 'string', description: 'Event description' },
        start: { type: 'string', description: 'Start time (ISO 8601)' },
        end: { type: 'string', description: 'End time (ISO 8601)' },
        attendees: { type: 'array', items: { type: 'string' }, description: 'Attendee email addresses' },
        location: { type: 'string', description: 'Event location' },
      },
      required: ['summary', 'start', 'end'],
    },
  },
  {
    name: 'calendar_update_event',
    description: 'Update an existing calendar event.',
    inputSchema: {
      type: 'object',
      properties: {
        eventId: { type: 'string', description: 'Event ID' },
        summary: { type: 'string', description: 'New event title' },
        description: { type: 'string', description: 'New description' },
        start: { type: 'string', description: 'New start time' },
        end: { type: 'string', description: 'New end time' },
      },
      required: ['eventId'],
    },
  },
  {
    name: 'calendar_delete_event',
    description: 'Delete a calendar event.',
    inputSchema: {
      type: 'object',
      properties: {
        eventId: { type: 'string', description: 'Event ID to delete' },
        sendNotifications: { type: 'boolean', default: true, description: 'Notify attendees' },
      },
      required: ['eventId'],
    },
  },

  // === Gmail Tools ===
  {
    name: 'gmail_search',
    description: 'Search emails in Gmail.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Gmail search query (supports from:, to:, subject:, has:attachment, etc.)' },
        maxResults: { type: 'integer', minimum: 1, maximum: 100, default: 10 },
        labelIds: { type: 'array', items: { type: 'string' }, description: 'Filter by label IDs' },
      },
      required: ['query'],
    },
  },
  {
    name: 'gmail_read_message',
    description: 'Read a specific email message.',
    inputSchema: {
      type: 'object',
      properties: {
        messageId: { type: 'string', description: 'Message ID' },
        format: { type: 'string', enum: ['full', 'metadata', 'minimal'], default: 'full' },
      },
      required: ['messageId'],
    },
  },
  {
    name: 'gmail_send',
    description: 'Send an email.',
    inputSchema: {
      type: 'object',
      properties: {
        to: { type: 'array', items: { type: 'string' }, description: 'Recipient email addresses' },
        cc: { type: 'array', items: { type: 'string' }, description: 'CC recipients' },
        bcc: { type: 'array', items: { type: 'string' }, description: 'BCC recipients' },
        subject: { type: 'string', description: 'Email subject' },
        body: { type: 'string', description: 'Email body (plain text or HTML)' },
        isHtml: { type: 'boolean', default: false, description: 'Whether body is HTML' },
        replyToMessageId: { type: 'string', description: 'Message ID to reply to' },
      },
      required: ['to', 'subject', 'body'],
    },
  },
  {
    name: 'gmail_create_draft',
    description: 'Create an email draft.',
    inputSchema: {
      type: 'object',
      properties: {
        to: { type: 'array', items: { type: 'string' }, description: 'Recipient email addresses' },
        subject: { type: 'string', description: 'Email subject' },
        body: { type: 'string', description: 'Email body' },
      },
      required: ['to', 'subject', 'body'],
    },
  },

  // === Jira Tools ===
  {
    name: 'jira_search_issues',
    description: 'Search for Jira issues using JQL.',
    inputSchema: {
      type: 'object',
      properties: {
        jql: { type: 'string', description: 'JQL query (e.g., "project = PROJ AND status = Open")' },
        maxResults: { type: 'integer', minimum: 1, maximum: 100, default: 50 },
        fields: { type: 'array', items: { type: 'string' }, description: 'Fields to return' },
      },
      required: ['jql'],
    },
  },
  {
    name: 'jira_get_issue',
    description: 'Get details of a specific Jira issue.',
    inputSchema: {
      type: 'object',
      properties: {
        issueKey: { type: 'string', description: 'Issue key (e.g., "PROJ-123")' },
        expand: { type: 'array', items: { type: 'string' }, description: 'Fields to expand (e.g., ["changelog", "comments"])' },
      },
      required: ['issueKey'],
    },
  },
  {
    name: 'jira_create_issue',
    description: 'Create a new Jira issue.',
    inputSchema: {
      type: 'object',
      properties: {
        projectKey: { type: 'string', description: 'Project key' },
        issueType: { type: 'string', description: 'Issue type (Bug, Task, Story, Epic)' },
        summary: { type: 'string', description: 'Issue summary/title' },
        description: { type: 'string', description: 'Issue description' },
        priority: { type: 'string', enum: ['Highest', 'High', 'Medium', 'Low', 'Lowest'], default: 'Medium' },
        assignee: { type: 'string', description: 'Assignee username' },
        labels: { type: 'array', items: { type: 'string' }, description: 'Issue labels' },
      },
      required: ['projectKey', 'issueType', 'summary'],
    },
  },
  {
    name: 'jira_update_issue',
    description: 'Update an existing Jira issue.',
    inputSchema: {
      type: 'object',
      properties: {
        issueKey: { type: 'string', description: 'Issue key' },
        fields: { type: 'object', description: 'Fields to update' },
        transition: { type: 'string', description: 'Transition to apply (e.g., "Done", "In Progress")' },
      },
      required: ['issueKey'],
    },
  },
  {
    name: 'jira_add_comment',
    description: 'Add a comment to a Jira issue.',
    inputSchema: {
      type: 'object',
      properties: {
        issueKey: { type: 'string', description: 'Issue key' },
        body: { type: 'string', description: 'Comment text' },
      },
      required: ['issueKey', 'body'],
    },
  },

  // === GitHub Tools ===
  {
    name: 'github_search_repos',
    description: 'Search GitHub repositories.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search query' },
        sort: { type: 'string', enum: ['stars', 'forks', 'updated'], default: 'stars' },
        limit: { type: 'integer', minimum: 1, maximum: 100, default: 10 },
      },
      required: ['query'],
    },
  },
  {
    name: 'github_list_issues',
    description: 'List issues in a GitHub repository.',
    inputSchema: {
      type: 'object',
      properties: {
        owner: { type: 'string', description: 'Repository owner' },
        repo: { type: 'string', description: 'Repository name' },
        state: { type: 'string', enum: ['open', 'closed', 'all'], default: 'open' },
        labels: { type: 'array', items: { type: 'string' }, description: 'Filter by labels' },
        limit: { type: 'integer', minimum: 1, maximum: 100, default: 30 },
      },
      required: ['owner', 'repo'],
    },
  },
  {
    name: 'github_create_issue',
    description: 'Create a new GitHub issue.',
    inputSchema: {
      type: 'object',
      properties: {
        owner: { type: 'string', description: 'Repository owner' },
        repo: { type: 'string', description: 'Repository name' },
        title: { type: 'string', description: 'Issue title' },
        body: { type: 'string', description: 'Issue body' },
        labels: { type: 'array', items: { type: 'string' }, description: 'Labels to apply' },
        assignees: { type: 'array', items: { type: 'string' }, description: 'Assignee usernames' },
      },
      required: ['owner', 'repo', 'title'],
    },
  },
  {
    name: 'github_list_prs',
    description: 'List pull requests in a GitHub repository.',
    inputSchema: {
      type: 'object',
      properties: {
        owner: { type: 'string', description: 'Repository owner' },
        repo: { type: 'string', description: 'Repository name' },
        state: { type: 'string', enum: ['open', 'closed', 'all'], default: 'open' },
        limit: { type: 'integer', minimum: 1, maximum: 100, default: 30 },
      },
      required: ['owner', 'repo'],
    },
  },
];

// Tool handlers
async function handleTool(name: string, args: Record<string, any>): Promise<any> {
  const timestamp = now();

  switch (name) {
    // Google Drive
    case 'gdrive_search': {
      const query = (args.query || '').toLowerCase();
      const limit = args.limit || 10;
      const results = fakeFiles.filter(f =>
        f.name.toLowerCase().includes(query) ||
        f.owner.toLowerCase().includes(query)
      ).slice(0, limit);
      return { success: true, files: results, totalResults: results.length, query: args.query };
    }

    case 'gdrive_read_file': {
      const file = fakeFiles.find(f => f.id === args.fileId);
      if (!file) return { success: false, error: `File not found: ${args.fileId}` };
      return {
        success: true,
        file: file,
        content: `[Simulated content for ${file.name}]\n\nThis is placeholder content representing the file "${file.name}".\nIn a real implementation, this would contain the actual file contents.`,
      };
    }

    case 'gdrive_create_file': {
      const newFile = {
        id: generateId('FILE'),
        name: args.name,
        mimeType: args.mimeType || 'text/plain',
        size: (args.content || '').length,
        modifiedTime: timestamp,
        owner: 'you@company.com',
      };
      return { success: true, file: newFile, message: 'File created successfully' };
    }

    case 'gdrive_share_file': {
      return {
        success: true,
        fileId: args.fileId,
        sharedWith: args.email,
        role: args.role || 'reader',
        permissionId: generateId('PERM'),
        message: `File shared with ${args.email} as ${args.role || 'reader'}`,
      };
    }

    // Google Sheets
    case 'sheets_read': {
      const sheet = fakeSpreadsheets[args.spreadsheetId];
      if (!sheet) return { success: false, error: `Spreadsheet not found: ${args.spreadsheetId}` };
      const sheetData = sheet.sheets[0];
      return {
        success: true,
        spreadsheetId: args.spreadsheetId,
        range: args.range,
        values: sheetData.data,
        majorDimension: 'ROWS',
      };
    }

    case 'sheets_write': {
      return {
        success: true,
        spreadsheetId: args.spreadsheetId,
        updatedRange: args.range,
        updatedRows: (args.values || []).length,
        updatedColumns: (args.values?.[0] || []).length,
        updatedCells: (args.values || []).flat().length,
      };
    }

    case 'sheets_append': {
      return {
        success: true,
        spreadsheetId: args.spreadsheetId,
        tableRange: args.range,
        updates: {
          updatedRange: `${args.range.split('!')[0]}!A${Math.floor(Math.random() * 100) + 10}`,
          updatedRows: (args.values || []).length,
          updatedCells: (args.values || []).flat().length,
        },
      };
    }

    case 'sheets_create': {
      const newId = generateId('SHEET');
      return {
        success: true,
        spreadsheetId: newId,
        spreadsheetUrl: `https://docs.google.com/spreadsheets/d/${newId}`,
        title: args.title,
        sheets: (args.sheetNames || ['Sheet1']).map((name: string, i: number) => ({
          sheetId: i,
          title: name,
        })),
      };
    }

    // Salesforce
    case 'salesforce_query': {
      const soql = (args.soql || '').toLowerCase();
      let records: any[] = [];

      if (soql.includes('account')) {
        records = fakeCompanies.map(c => ({ Id: c.id, Name: c.name, Industry: c.industry, AnnualRevenue: c.revenue }));
      } else if (soql.includes('opportunity')) {
        records = fakeOpportunities.map(o => ({ Id: o.id, Name: o.name, StageName: o.stage, Amount: o.amount, CloseDate: o.closeDate }));
      } else if (soql.includes('contact') || soql.includes('user')) {
        records = fakeUsers.map(u => ({ Id: u.id, Name: u.name, Email: u.email, Department: u.department }));
      }

      return {
        success: true,
        totalSize: records.length,
        done: true,
        records: records.slice(0, args.limit || 100),
      };
    }

    case 'salesforce_get_record': {
      let record: any = null;
      if (args.objectType === 'Account') {
        record = fakeCompanies.find(c => c.id === args.recordId);
      } else if (args.objectType === 'Opportunity') {
        record = fakeOpportunities.find(o => o.id === args.recordId);
      }
      if (!record) return { success: false, error: `Record not found: ${args.recordId}` };
      return { success: true, record };
    }

    case 'salesforce_create_record': {
      const newId = generateId(args.objectType?.substring(0, 3).toUpperCase() || 'REC');
      return {
        success: true,
        id: newId,
        objectType: args.objectType,
        message: `${args.objectType} created successfully`,
      };
    }

    case 'salesforce_update_record': {
      return {
        success: true,
        id: args.recordId,
        objectType: args.objectType,
        updatedFields: Object.keys(args.data || {}),
        message: `${args.objectType} updated successfully`,
      };
    }

    case 'salesforce_search': {
      const term = (args.searchTerm || '').toLowerCase();
      const results: any[] = [];
      fakeCompanies.filter(c => c.name.toLowerCase().includes(term)).forEach(c => results.push({ type: 'Account', ...c }));
      fakeUsers.filter(u => u.name.toLowerCase().includes(term)).forEach(u => results.push({ type: 'Contact', ...u }));
      return { success: true, searchRecords: results.slice(0, args.limit || 20) };
    }

    // Slack
    case 'slack_send_message': {
      return {
        success: true,
        ok: true,
        channel: args.channel,
        ts: `${Date.now() / 1000}.000100`,
        message: { text: args.text, user: 'U001', ts: `${Date.now() / 1000}.000100` },
      };
    }

    case 'slack_get_messages': {
      const channelId = args.channel.startsWith('#') ? fakeSlackChannels.find(c => c.name === args.channel.slice(1))?.id : args.channel;
      const messages = fakeSlackMessages.filter(m => m.channel === channelId).slice(0, args.limit || 20);
      return { success: true, ok: true, messages, hasMore: false };
    }

    case 'slack_search_messages': {
      const query = (args.query || '').toLowerCase();
      const matches = fakeSlackMessages.filter(m => m.text.toLowerCase().includes(query));
      return {
        success: true,
        ok: true,
        query: args.query,
        messages: { total: matches.length, matches: matches.slice(0, args.limit || 20) },
      };
    }

    case 'slack_list_channels': {
      return { success: true, ok: true, channels: fakeSlackChannels };
    }

    case 'slack_get_user_info': {
      const user = fakeUsers.find(u => u.id === args.userId);
      if (!user) return { success: false, ok: false, error: 'user_not_found' };
      return { success: true, ok: true, user: { ...user, realName: user.name, displayName: user.name.split(' ')[0] } };
    }

    case 'slack_set_status': {
      return {
        success: true,
        ok: true,
        profile: {
          statusText: args.statusText,
          statusEmoji: args.statusEmoji || ':speech_balloon:',
          statusExpiration: args.expirationMinutes ? Date.now() + args.expirationMinutes * 60000 : 0,
        },
      };
    }

    // Calendar
    case 'calendar_list_events': {
      return { success: true, items: fakeCalendarEvents };
    }

    case 'calendar_create_event': {
      const newEvent = {
        id: generateId('EVT'),
        summary: args.summary,
        description: args.description,
        start: args.start,
        end: args.end,
        attendees: args.attendees || [],
        htmlLink: `https://calendar.google.com/event?eid=${generateId('E')}`,
      };
      return { success: true, event: newEvent };
    }

    case 'calendar_update_event': {
      return {
        success: true,
        event: {
          id: args.eventId,
          ...(args.summary && { summary: args.summary }),
          ...(args.description && { description: args.description }),
          ...(args.start && { start: args.start }),
          ...(args.end && { end: args.end }),
          updated: timestamp,
        },
      };
    }

    case 'calendar_delete_event': {
      return { success: true, deleted: true, eventId: args.eventId };
    }

    // Gmail
    case 'gmail_search': {
      const query = (args.query || '').toLowerCase();
      const results = fakeEmails.filter(e =>
        e.subject.toLowerCase().includes(query) ||
        e.from.toLowerCase().includes(query) ||
        e.snippet.toLowerCase().includes(query)
      );
      return { success: true, messages: results.slice(0, args.maxResults || 10), resultSizeEstimate: results.length };
    }

    case 'gmail_read_message': {
      const email = fakeEmails.find(e => e.id === args.messageId);
      if (!email) return { success: false, error: `Message not found: ${args.messageId}` };
      return {
        success: true,
        message: {
          ...email,
          body: `Full body of email: "${email.subject}"\n\n${email.snippet}\n\n[Additional content would appear here in a real implementation]`,
        },
      };
    }

    case 'gmail_send': {
      return {
        success: true,
        id: generateId('MSG'),
        threadId: generateId('THR'),
        labelIds: ['SENT'],
        message: `Email sent to ${(args.to || []).join(', ')}`,
      };
    }

    case 'gmail_create_draft': {
      return {
        success: true,
        id: generateId('DRF'),
        message: { id: generateId('MSG'), threadId: generateId('THR') },
      };
    }

    // Jira
    case 'jira_search_issues': {
      const issues = [
        { key: 'PROJ-101', summary: 'Implement user authentication', status: 'In Progress', priority: 'High', assignee: 'alice' },
        { key: 'PROJ-102', summary: 'Fix login page CSS', status: 'Open', priority: 'Medium', assignee: 'emma' },
        { key: 'PROJ-103', summary: 'Add API rate limiting', status: 'Done', priority: 'High', assignee: 'alice' },
      ];
      return { success: true, issues, total: issues.length, maxResults: args.maxResults || 50 };
    }

    case 'jira_get_issue': {
      return {
        success: true,
        key: args.issueKey,
        fields: {
          summary: `Issue ${args.issueKey}`,
          status: { name: 'In Progress' },
          priority: { name: 'High' },
          assignee: { displayName: 'Alice Johnson' },
          description: 'Detailed description of the issue...',
          created: '2026-01-15T10:00:00Z',
          updated: timestamp,
        },
      };
    }

    case 'jira_create_issue': {
      const issueKey = `${args.projectKey}-${Math.floor(Math.random() * 900) + 100}`;
      return {
        success: true,
        id: generateId(''),
        key: issueKey,
        self: `https://your-domain.atlassian.net/rest/api/2/issue/${issueKey}`,
      };
    }

    case 'jira_update_issue': {
      return { success: true, key: args.issueKey, updated: true };
    }

    case 'jira_add_comment': {
      return {
        success: true,
        id: generateId('CMT'),
        issueKey: args.issueKey,
        body: args.body,
        created: timestamp,
      };
    }

    // GitHub
    case 'github_search_repos': {
      const repos = [
        { fullName: 'facebook/react', description: 'A declarative UI library', stars: 220000, language: 'JavaScript' },
        { fullName: 'microsoft/vscode', description: 'Visual Studio Code', stars: 155000, language: 'TypeScript' },
        { fullName: 'torvalds/linux', description: 'Linux kernel source tree', stars: 165000, language: 'C' },
      ];
      return { success: true, totalCount: repos.length, items: repos.slice(0, args.limit || 10) };
    }

    case 'github_list_issues': {
      const issues = [
        { number: 1234, title: 'Bug in component rendering', state: 'open', labels: ['bug'], user: 'contributor1' },
        { number: 1235, title: 'Feature request: dark mode', state: 'open', labels: ['enhancement'], user: 'contributor2' },
      ];
      return { success: true, issues };
    }

    case 'github_create_issue': {
      return {
        success: true,
        number: Math.floor(Math.random() * 9000) + 1000,
        title: args.title,
        htmlUrl: `https://github.com/${args.owner}/${args.repo}/issues/${Math.floor(Math.random() * 9000) + 1000}`,
      };
    }

    case 'github_list_prs': {
      const prs = [
        { number: 567, title: 'Fix memory leak in worker', state: 'open', user: 'dev1', draft: false },
        { number: 568, title: 'Add TypeScript support', state: 'open', user: 'dev2', draft: true },
      ];
      return { success: true, pullRequests: prs };
    }

    default:
      return { error: `Unknown tool: ${name}` };
  }
}

// Server setup
const server = new Server(
  { name: 'mcp-harness', version: '1.0.0' },
  { capabilities: { tools: {} } }
);

server.setRequestHandler(ListToolsRequestSchema, async () => {
  return { tools };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const toolName = request.params.name;
  const args = (request.params.arguments || {}) as Record<string, any>;

  const result = await handleTool(toolName, args);

  // Log the tool call
  logToolCall(toolName, args, result);

  return {
    content: [
      {
        type: 'text',
        text: JSON.stringify(result, null, 2),
      },
    ],
  };
});

const transport = new StdioServerTransport();
await server.connect(transport);
