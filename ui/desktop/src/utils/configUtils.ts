export const configLabels: Record<string, string> = {
  // goose settings
  GOOSE_PROVIDER: 'GOOSE_PROVIDER',
  GOOSE_MODEL: 'GOOSE_MODEL',
  GOOSE_TEMPERATURE: 'GOOSE_TEMPERATURE',
  GOOSE_MODE: 'GOOSE_MODE',
  GOOSE_LEAD_PROVIDER: 'GOOSE_LEAD_PROVIDER',
  GOOSE_LEAD_MODEL: 'GOOSE_LEAD_MODEL',
  GOOSE_PLANNER_PROVIDER: 'GOOSE_PLANNER_PROVIDER',
  GOOSE_PLANNER_MODEL: 'GOOSE_PLANNER_MODEL',
  GOOSE_TOOLSHIM: 'GOOSE_TOOLSHIM',
  GOOSE_TOOLSHIM_OLLAMA_MODEL: 'GOOSE_TOOLSHIM_OLLAMA_MODEL',
  GOOSE_CLI_MIN_PRIORITY: 'GOOSE_CLI_MIN_PRIORITY',
  GOOSE_ALLOWLIST: 'GOOSE_ALLOWLIST',
  GOOSE_RECIPE_GITHUB_REPO: 'GOOSE_RECIPE_GITHUB_REPO',

  // openai
  OPENAI_API_KEY: 'OPENAI_API_KEY',
  OPENAI_HOST: 'OPENAI_HOST',
  OPENAI_BASE_PATH: 'OPENAI_BASE_PATH',

  // groq
  GROQ_API_KEY: 'GROQ_API_KEY',

  // openrouter
  OPENROUTER_API_KEY: 'OPENROUTER_API_KEY',

  // anthropic
  ANTHROPIC_API_KEY: 'ANTHROPIC_API_KEY',
  ANTHROPIC_HOST: 'ANTHROPIC_HOST',

  // google
  GOOGLE_API_KEY: 'GOOGLE_API_KEY',

  // databricks
  DATABRICKS_HOST: 'DATABRICKS_HOST',

  // ollama
  OLLAMA_HOST: 'OLLAMA_HOST',

  // azure openai
  AZURE_OPENAI_API_KEY: 'AZURE_OPENAI_API_KEY',
  AZURE_OPENAI_ENDPOINT: 'AZURE_OPENAI_ENDPOINT',
  AZURE_OPENAI_DEPLOYMENT_NAME: 'AZURE_OPENAI_DEPLOYMENT_NAME',
  AZURE_OPENAI_API_VERSION: 'AZURE_OPENAI_API_VERSION',

  // gcp vertex
  GCP_PROJECT_ID: 'GCP_PROJECT_ID',
  GCP_LOCATION: 'GCP_LOCATION',

  // snowflake
  SNOWFLAKE_HOST: 'SNOWFLAKE_HOST',
  SNOWFLAKE_TOKEN: 'SNOWFLAKE_TOKEN',
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
