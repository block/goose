const { McpServer } = require("@modelcontextprotocol/sdk/server/mcp.js");
const { StdioServerTransport } = require("@modelcontextprotocol/sdk/server/stdio.js");

// Collection of running-related inspirational quotes
const runningQuotes = [
    {
        quote: "The miracle isn't that I finished. The miracle is that I had the courage to start.",
        author: "John Bingham"
    },
    {
        quote: "Running is the greatest metaphor for life, because you get out of it what you put into it.",
        author: "Oprah Winfrey"
    },
    {
        quote: "Pain is temporary. Quitting lasts forever.",
        author: "Lance Armstrong"
    },
    {
        quote: "Run when you can, walk if you have to, crawl if you must; just never give up.",
        author: "Dean Karnazes"
    },
    {
        quote: "The only bad workout is the one that didn't happen.",
        author: "Unknown"
    },
    {
        quote: "Whether you think you can or think you can't, you're right.",
        author: "Henry Ford"
    },
    {
        quote: "If you want to run, run a mile. If you want to experience a different life, run a marathon.",
        author: "Emil Zatopek"
    },
    {
        quote: "The voice inside your head that says you can't do this is a liar.",
        author: "Unknown"
    }
];

async function startServer() {
    const server = new McpServer({
        name: "Running Quotes",
        version: "1.0.0"
    });

    server.tool("runningQuote",
      "Generates an inspirational running quote",
      async () => {
          const randomQuote = runningQuotes[Math.floor(Math.random() * runningQuotes.length)];
          return {
              content: [{
                  type: "text",
                  text: `"${randomQuote.quote}" - ${randomQuote.author}`
              }]
          };
      }
    );

    // Start receiving messages on stdin and sending messages on stdout
    const transport = new StdioServerTransport();
    await server.connect(transport);
}

// Only start the server if this is the main module
if (require.main === module) {
    startServer().catch(console.error);
}

module.exports = {
    runningQuotes
};
