export const configLabels: Record<string, string> = {
  // goose settings
  GOOSE_PROVIDER: 'Provider',
  GOOSE_MODEL: 'Model',
  GOOSE_TEMPERATURE: 'Temperature',
  GOOSE_MODE: 'Mode',
  GOOSE_LEAD_PROVIDER: 'Lead Provider',
  GOOSE_LEAD_MODEL: 'Lead Model',
  GOOSE_PLANNER_PROVIDER: 'Planner Provider',
  GOOSE_PLANNER_MODEL: 'Planner Model',
  GOOSE_TOOLSHIM: 'Tool Shim',
  GOOSE_TOOLSHIM_OLLAMA_MODEL: 'Tool Shim Ollama Model',
  GOOSE_CLI_MIN_PRIORITY: 'CLI Min Priority',
  GOOSE_ALLOWLIST: 'Allow List',
  GOOSE_RECIPE_GITHUB_REPO: 'Recipe GitHub Repo',

  // security settings
  SECURITY_PROMPT_ENABLED: 'Prompt Injection Detection Enabled',
  SECURITY_PROMPT_THRESHOLD: 'Prompt Injection Detection Threshold',
  SECURITY_PROMPT_CLASSIFIER_ENABLED: 'ML-based Prompt Injection Detection Enabled',
  SECURITY_PROMPT_CLASSIFIER_MODEL: 'ML-based Prompt Injection Detection Model',
  SECURITY_PROMPT_CLASSIFIER_ENDPOINT: 'ML Classification Endpoint',
  SECURITY_PROMPT_CLASSIFIER_TOKEN: 'ML Classification API Token',

  // openai
  OPENAI_API_KEY: 'OpenAI API Key',
  OPENAI_HOST: 'OpenAI Host',
  OPENAI_BASE_PATH: 'OpenAI Base Path',

  // groq
  GROQ_API_KEY: 'Groq API Key',

  // openrouter
  OPENROUTER_API_KEY: 'OpenRouter API Key',

  // anthropic
  ANTHROPIC_API_KEY: 'Anthropic API Key',
  ANTHROPIC_HOST: 'Anthropic Host',

  // google
  GOOGLE_API_KEY: 'Google API Key',

  // databricks
  DATABRICKS_HOST: 'Databricks Host',

  // ollama
  OLLAMA_HOST: 'Ollama Host',

  // azure openai
  AZURE_OPENAI_API_KEY: 'Azure OpenAI API Key',
  AZURE_OPENAI_ENDPOINT: 'Azure OpenAI Endpoint',
  AZURE_OPENAI_DEPLOYMENT_NAME: 'Azure OpenAI Deployment Name',
  AZURE_OPENAI_API_VERSION: 'Azure OpenAI API Version',
  AZURE_OPENAI_TENANT_ID: 'Azure AD Tenant ID',
  AZURE_OPENAI_CLIENT_ID: 'Azure AD Client ID',
  AZURE_OPENAI_CLIENT_SECRET: 'Azure AD Client Secret',
  AZURE_OPENAI_CERTIFICATE_PATH: 'Certificate Path',
  AZURE_OPENAI_CERTIFICATE: 'Certificate (PEM)',
  AZURE_OPENAI_USE_MANAGED_IDENTITY: 'Use Managed Identity',
  AZURE_OPENAI_TOKEN_SCOPE: 'Token Scope',

  // openai with azure entra id
  OPENAI_AZURE_TENANT_ID: 'Azure AD Tenant ID',
  OPENAI_AZURE_CLIENT_ID: 'Azure AD Client ID',
  OPENAI_AZURE_CLIENT_SECRET: 'Azure AD Client Secret',
  OPENAI_AZURE_CERTIFICATE_PATH: 'Certificate Path',
  OPENAI_AZURE_CERTIFICATE: 'Certificate (PEM)',
  OPENAI_AZURE_USE_MANAGED_IDENTITY: 'Use Managed Identity',
  OPENAI_AZURE_TOKEN_SCOPE: 'Token Scope',

  // gcp vertex
  GCP_PROJECT_ID: 'GCP Project ID',
  GCP_LOCATION: 'GCP Location',

  // snowflake
  SNOWFLAKE_HOST: 'Snowflake Host',
  SNOWFLAKE_TOKEN: 'Snowflake Token',
};

export const providerPrefixes: Record<string, string[]> = {
  openai: ['OPENAI_', 'OPENAI_AZURE_'],
  anthropic: ['ANTHROPIC_'],
  google: ['GOOGLE_'],
  groq: ['GROQ_'],
  databricks: ['DATABRICKS_'],
  openrouter: ['OPENROUTER_'],
  ollama: ['OLLAMA_'],
  azure_openai: ['AZURE_OPENAI_'],
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
