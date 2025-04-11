export type EnvironmentVariable = {
  name: string;
  description: string;
  required: boolean;
};

export type Extension = {
  name: string;
  command?: string;
  url?: string;
  is_builtin: boolean;
  link?: string;
  installation_notes?: string;
  environmentVariables?: EnvironmentVariable[];
};

export type Prompt = {
  id: string;
  title: string;
  description: string;
  example_prompt: string;
  extensions: Extension[];
};