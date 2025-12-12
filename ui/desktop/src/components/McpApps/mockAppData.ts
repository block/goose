type UIResourceUri = `ui://${string}`;
type UIResourceMimeType = `text/html;profile=mcp-app`;

interface UIResourceMeta {
  csp?: {
    connectDomains?: string[];
    resourceDomains?: string[];
  };
  domain?: `https://${string}`;
  prefersBorder?: boolean;
}

interface MockResourceListItem {
  uri: UIResourceUri;
  name: string;
  description: string;
  mimeType: UIResourceMimeType;
}

interface MockReadResourceItem {
  uri: UIResourceUri;
  description?: string;
  mimeType: UIResourceMimeType;
  text?: string;
  _meta?: {
    ui?: UIResourceMeta;
  };
}

interface MockListedResources {
  resources: MockResourceListItem[];
}

interface MockReadResources {
  contents: MockReadResourceItem[];
}

const UI_RESOURCE_URI = 'ui://weather-server/dashboard-template' as const;

export const mockToolListResult = {
  name: 'get_weather',
  description: 'Get current weather for a location',
  inputSchema: {
    type: 'object',
    properties: {
      location: { type: 'string' },
    },
  },
  _meta: {
    'ui/resourceUri': UI_RESOURCE_URI,
  },
};

export const mockResourceListResult: MockListedResources = {
  resources: [
    {
      uri: UI_RESOURCE_URI,
      name: 'weather_dashboard',
      description: 'Interactive weather dashboard widget',
      mimeType: 'text/html;profile=mcp-app',
    },
  ],
};

const mockAppHtml = `<!DOCTYPE html>
<html>
<head>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&display=swap" rel="stylesheet">
  <style>
    html {
      overflow: hidden;
    }
    body {
      margin: 0;
      padding: 16px;
      color: white;
      background-color: #2563eb;
      font-family: "Instrument Serif", system-ui, sans-serif;
      font-weight: 400;
      font-style: normal;
    }
    h1 {
      font-size: 10rem;
      text-align: center;
      line-height: 1;
    }
  </style>
</head>
<body>
  <h1>Goose <br /> MCP Apps</h1>
  <p>This content will resize and notify the host.</p>
  <script>
    (function() {
      function sendSizeChanged() {
        const width = document.body.scrollWidth;
        const height = document.body.scrollHeight;
        window.parent.postMessage({
          jsonrpc: '2.0',
          method: 'ui/notifications/size-changed',
          params: { width, height }
        }, '*');
      }

      // Send initial size
      sendSizeChanged();

      // Observe size changes
      const resizeObserver = new ResizeObserver(sendSizeChanged);
      resizeObserver.observe(document.body);
    })();
  </script>
</body>
</html>`;

export const mockResourceReadResult: MockReadResources = {
  contents: [
    {
      uri: UI_RESOURCE_URI,
      mimeType: 'text/html;profile=mcp-app',
      text: mockAppHtml,
      _meta: {
        ui: {
          csp: {
            connectDomains: ['https://api.openweathermap.org'],
            resourceDomains: ['https://fonts.googleapis.com', 'https://fonts.gstatic.com'],
          },
          prefersBorder: true,
        },
      },
    },
  ],
};
