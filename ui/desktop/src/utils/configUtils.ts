export const configLabels: Record<string, string> = {
  // goose settings
  GOOSE_PROVIDER: 'Goose Provider',
  GOOSE_MODEL: 'Goose Model',
  GOOSE_TEMPERATURE: 'Goose Temperature',
  GOOSE_MODE: 'Goose Execution Mode',
  GOOSE_LEAD_PROVIDER: 'Goose Lead Provider',
  GOOSE_LEAD_MODEL: 'Goose Lead Model',
  GOOSE_PLANNER_PROVIDER: 'Goose Planner Provider',
  GOOSE_PLANNER_MODEL: 'Goose Planner Model',
  GOOSE_TOOLSHIM: 'Goose Tool Shim Enabled',
  GOOSE_TOOLSHIM_OLLAMA_MODEL: 'Goose Tool Shim Model',
  GOOSE_CLI_MIN_PRIORITY: 'Goose Minimum Priority',
  GOOSE_ALLOWLIST: 'Goose Allowlist URL',
  GOOSE_RECIPE_GITHUB_REPO: 'Goose Recipe Repository',

  // openai
  OPENAI_HOST: 'OpenAI API Host',
  OPENAI_BASE_PATH: 'OpenAI Base Path',

  // anthropic
  ANTHROPIC_HOST: 'Anthropic API Host',

  // databricks
  DATABRICKS_HOST: 'Databricks Host URL',

  // ollama
  OLLAMA_HOST: 'Ollama Server Host',

  // azure openai
  AZURE_OPENAI_ENDPOINT: 'Azure OpenAI Endpoint',
  AZURE_OPENAI_DEPLOYMENT_NAME: 'Azure OpenAI Deployment Name',
  AZURE_OPENAI_API_VERSION: 'Azure OpenAI API Version',

  // gcp vertex
  GCP_PROJECT_ID: 'Project ID',
  GCP_LOCATION: 'Location',

  // snowflake
  SNOWFLAKE_HOST: 'Snowflake Account Host',
};

export const providerPrefixes: Record<string, string[]> = {
  openai: ['OPENAI_'],
  anthropic: ['ANTHROPIC_'],
  google: ['GOOGLE_'],
  groq: ['GROQ_'],
  databricks: ['DATABRICKS_'],
  openrouter: ['OPENROUTER_'],
  ollama: ['OLLAMA_'],
  azure_openai: ['AZURE_'],
  gcp_vertex_ai: ['GCP_'],
  snowflake: ['SNOWFLAKE_'],
};

export const getUiNames = (key: string): string => {
  if (configLabels[key]) {
    return configLabels[key];
  }
  return key
    .split('_')
    .map((word) => word.charAt(0) + word.slice(1).toLowerCase())
    .join(' ');
};
