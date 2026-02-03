export interface MCPIntegration {
  name: string;
  iconUrl: string;
  description: string;
  tools: {
    examples: string[];
    descriptions?: string[];
    count: string;
  };
  docsUrl?: string;
  repoUrl?: string;
  categories: string[];
  provider: string;
  authType?: string;
}

export const mcpIntegrations: MCPIntegration[] = [
  {
    name: "GitHub",
    iconUrl: "https://cdn.simpleicons.org/github",
    description:
      "Official hosted MCP for GitHub; supports OAuth or PAT; capabilities organized into 15 toolsets (repos, PRs, issues, actions, security, etc.)",
    tools: {
      examples: [
        "repos",
        "pull_requests",
        "issues",
        "actions",
        "code_security",
        "dependabot",
        "discussions",
        "gists",
        "notifications",
        "orgs",
        "users",
      ],
      descriptions: [
        "Manage repositories, branches, and commits",
        "Create, review, and merge pull requests",
        "Track and manage project issues",
        "Monitor and control GitHub Actions workflows",
        "Scan code for vulnerabilities and security issues",
        "Configure automated dependency updates",
        "Engage in team discussions and Q&A",
        "Create and manage code snippets",
        "Stay updated with repository activities",
        "Manage organization settings and teams",
        "Access user profiles and contributions",
      ],
      count: "11",
    },
    docsUrl:
      "https://docs.github.com/en/copilot/how-tos/provide-context/use-mcp/use-the-github-mcp-server",
    repoUrl: "https://github.com/github/github-mcp-server",
    categories: ["Development", "Version Control"],
    provider: "GitHub",
    authType: "OAuth/PAT",
  },
  {
    name: "Notion",
    iconUrl: "https://cdn.simpleicons.org/notion",
    description:
      "Notion-hosted remote MCP with OAuth; lets AI search, read, create & update workspace content.",
    tools: {
      examples: [
        "search",
        "fetch",
        "create-pages",
        "update-page",
        "move-pages",
        "duplicate-page",
        "create-database",
        "update-database",
        "create-comment",
        "get-comments",
        "get-users",
        "get-user",
        "get-self",
      ],
      descriptions: [
        "Search across all workspace content",
        "Retrieve page and database content",
        "Create new pages with rich content",
        "Modify existing page properties and blocks",
        "Reorganize page hierarchy and structure",
        "Clone pages with all their content",
        "Set up new databases with custom properties",
        "Modify database schemas and views",
        "Add comments to pages and discussions",
        "Read and manage page comments",
        "List all workspace members",
        "Get specific user details",
        "Retrieve current user information",
      ],
      count: "13",
    },
    docsUrl: "https://developers.notion.com/docs/mcp",
    categories: ["Productivity", "Documentation"],
    provider: "Notion",
    authType: "OAuth",
  },
  {
    name: "Sentry",
    iconUrl: "https://cdn.simpleicons.org/sentry",
    description:
      "Sentry-hosted remote MCP (OAuth), with issue context, search, & Seer integration.",
    tools: {
      examples: [
        "Organizations",
        "Projects",
        "Teams",
        "Issues",
        "DSNs",
        "Error Searching",
        "Seer",
        "Releases",
        "Performance",
        "Custom Queries",
      ],
      count: "16+",
    },
    docsUrl: "https://docs.sentry.io/product/sentry-mcp/",
    categories: ["Monitoring", "Error Tracking"],
    provider: "Sentry",
    authType: "OAuth",
  },
  {
    name: "Linear",
    iconUrl: "https://cdn.simpleicons.org/linear",
    description:
      "Linear-hosted remote MCP (OAuth 2.1) via SSE/HTTP; find/create/update issues, projects, comments.",
    tools: {
      examples: [
        "search issues",
        "create issues",
        "update issues",
        "manage projects",
        "handle comments",
      ],
      count: "22",
    },
    docsUrl: "https://linear.app/docs/mcp",
    categories: ["Project Management", "Issue Tracking"],
    provider: "Linear",
    authType: "OAuth 2.1",
  },
  {
    name: "Intercom",
    iconUrl: "https://cdn.simpleicons.org/intercom",
    description:
      "Intercom-hosted remote MCP (HTTP & legacy SSE); OAuth or bearer token; US workspaces currently.",
    tools: {
      examples: [
        "search",
        "fetch",
        "search_conversations",
        "get_conversation",
        "search_contacts",
        "get_contact",
      ],
      count: "6",
    },
    docsUrl: "https://developers.intercom.com/docs/guides/mcp",
    categories: ["Customer Support", "Communication"],
    provider: "Intercom",
    authType: "OAuth/Bearer Token",
  },
  {
    name: "Asana",
    iconUrl: "https://cdn.simpleicons.org/asana",
    description: "Asana remote MCP (beta) over SSE with OAuth; brings the Work Graph to AI tools.",
    tools: {
      examples: ["projects", "tasks", "users", "goals", "teams", "typeahead"],
      count: "30",
    },
    docsUrl: "https://developers.asana.com/docs/mcp-server",
    categories: ["Project Management", "Collaboration"],
    provider: "Asana",
    authType: "OAuth",
  },
  {
    name: "Neon",
    iconUrl: "https://neon.tech/favicon.ico",
    description:
      "Open-source MCP for Neon Postgres; managed remote at mcp.neon.tech supports OAuth & API keys; local mode too.",
    tools: {
      examples: ["list_projects", "describe_project", "create_project", "create_branch", "run_sql"],
      count: "21",
    },
    docsUrl: "https://neon.com/docs/ai/neon-mcp-server",
    repoUrl: "https://github.com/neondatabase/mcp-server-neon",
    categories: ["Database", "PostgreSQL"],
    provider: "Neon",
    authType: "OAuth/API Keys",
  },
  {
    name: "Atlassian",
    iconUrl: "https://cdn.simpleicons.org/atlassian",
    description:
      "Remote MCP Server (beta) with OAuth 2.1; surfaces Jira/Confluence/Compass into AI tools (rate-limited by plan).",
    tools: {
      examples: ["search pages", "summarize issues", "create issues", "update pages"],
      count: "not published",
    },
    docsUrl:
      "https://support.atlassian.com/rovo/docs/getting-started-with-the-atlassian-remote-mcp-server/",
    categories: ["Project Management", "Documentation"],
    provider: "Atlassian",
    authType: "OAuth 2.1",
  },
  {
    name: "Webflow",
    iconUrl: "https://cdn.simpleicons.org/webflow",
    description:
      "Official MCP (remote via OAuth or local); tools for sites, pages, components & CMS.",
    tools: {
      examples: [
        "sites list/get/publish",
        "pages get/update",
        "components list/get/update",
        "collections",
        "custom-code",
        "ask-webflow-ai",
      ],
      count: "20",
    },
    docsUrl: "https://developers.webflow.com/data/docs/ai-tools",
    repoUrl: "https://github.com/webflow/mcp-server",
    categories: ["Web Development", "CMS"],
    provider: "Webflow",
    authType: "OAuth",
  },
  {
    name: "Wix",
    iconUrl: "https://cdn.simpleicons.org/wix",
    description:
      "Official Wix MCP (remote helper @wix/mcp-remote to https://mcp.wix.com/sse); OAuth flow; broad Wix solutions.",
    tools: {
      examples: ["eCommerce", "Bookings", "Payments", "CMS", "CRM", "Blog", "Events"],
      count: "not published",
    },
    docsUrl: "https://www.wix.com/studio/developers/mcp-server",
    categories: ["Web Development", "eCommerce"],
    provider: "Wix",
    authType: "OAuth",
  },
];
