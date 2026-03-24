export interface AgentConfig {
  model: string;
  provider: string;
  /** Extensions (runner knows which are platform vs builtin) */
  extensions?: string[];
  /** Stdio extension commands (for custom MCP servers) */
  stdio?: string[];
  /** Path to goose binary (default: "goose") */
  "goose-bin"?: string;
  temperature?: number;
  maxTokens?: number;
}

export interface Scenario {
  name: string;
  description: string;
  prompt?: string;
  /** Files to create before running (relative paths) */
  setup?: Record<string, string>;
  /** Validation rules to check after agent completes (single-turn) */
  validate?: ValidationRule[];
  /** Multi-turn conversation (alternative to single prompt+validate) */
  turns?: Turn[];
  /** Tags for filtering scenarios */
  tags?: string[];
}

/** A single turn in a multi-turn conversation */
export interface Turn {
  /** The prompt for this turn */
  prompt: string;
  /** Validation rules to check after this turn completes */
  validate: ValidationRule[];
}

export type ValidationRule =
  | { type: "file_exists"; path: string; name?: string }
  | { type: "file_contains"; path: string; pattern: string; name?: string }
  | { type: "file_matches"; path: string; regex: string; name?: string }
  | { type: "file_not_matches"; path: string; regex: string; name?: string }
  | { type: "file_not_empty"; path: string; name?: string }
  | { type: "command_succeeds"; command: string; name?: string }
  | { type: "tool_called"; tool: string; args?: Record<string, string | RegExp>; name?: string }
  | { type: "custom"; fn: string; name?: string };

export interface TestRun {
  scenario: Scenario;
  config: AgentConfig;
  workdir: string;
  startTime: Date;
  endTime?: Date;
  status: "pending" | "running" | "passed" | "failed";
  errors?: string[];
}

export interface TestResult {
  run: TestRun;
  validations: Array<{
    rule: ValidationRule;
    passed: boolean;
    message?: string;
  }>;
}

export interface SuiteConfig {
  /** Agent configurations to permute */
  agents: AgentConfig[];
  /** Scenarios to run */
  scenarios: string[];
  /** Base directory for test workspaces */
  workdir: string;
  /** Parallel execution count */
  parallel?: number;
}
