import { MockListedResources, MockReadResources } from './types';

const UI_RESOURCE_URI = 'ui://weather-server/dashboard-template';

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

export const mockResourceReadResult: MockReadResources = {
  contents: [
    {
      uri: UI_RESOURCE_URI,
      mimeType: 'text/html;profile=mcp-app',
      text: '<!DOCTYPE html><html><body style="color: white; background-color: blue;">Hello, MCP App!</body></html>',
      _meta: {
        ui: {
          csp: {
            connectDomains: ['https://api.openweathermap.org'],
            resourceDomains: ['https://cdn.jsdelivr.net'],
          },
          prefersBorder: true,
        },
      },
    },
  ],
};
