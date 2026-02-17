#!/usr/bin/env node
import { mkdirSync, writeFileSync, rmSync, readdirSync, existsSync, copyFileSync } from "node:fs";
import { join, basename, dirname } from "node:path";
import { homedir } from "node:os";
import { execSync } from "node:child_process";
import { parse, stringify } from "yaml";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import type { Scenario, TestResult, TestRun, Turn } from "./types.js";
import { validateAll } from "./validator.js";

// =============================================================================
// Types
// =============================================================================

type RunnerType = "goose" | "opencode" | "pi";

interface ModelConfig {
  name: string;
  provider: string;
  model: string;
}

interface RunnerConfig {
  name: string;
  type: RunnerType;
  bin: string;
  extensions?: string[];  // goose-specific
  stdio?: string[];       // MCP servers
}

interface MatrixEntry {
  scenario: string;
  models?: string[];   // omit = all models
  runners?: string[];  // omit = all runners
}

interface SuiteConfig {
  models: ModelConfig[];
  runners: RunnerConfig[];
  matrix?: MatrixEntry[];
}

// A test pair: scenario × model × runner
interface TestPair {
  scenario: Scenario;
  model: ModelConfig;
  runner: RunnerConfig;
}

interface TestResultWithLog extends TestResult {
  logFile: string;
  runnerName: string;
  toolCalls: number;
  turns: number;
  cached?: boolean;
}

// =============================================================================
// Cache Types
// =============================================================================

interface CacheInputs {
  scenarioHash: string;
  modelKey: string;
  runnerHash: string;
  binaryHash: string;
  mcpHarnessHash: string;
}

interface CacheEntry {
  timestamp: string;
  inputs: CacheInputs;
  result: {
    status: "passed" | "failed";
    validations: Array<{ rule: any; passed: boolean; message?: string }>;
    duration: number;
    toolCalls: number;
    turns: number;
    errors?: string[];
  };
  logFile: string;
}

interface CacheIndex {
  version: number;
  entries: Record<string, CacheEntry>;
}

// =============================================================================
// Cache Utilities
// =============================================================================

const CACHE_DIR = join(import.meta.dirname, "../.cache");
const CACHE_INDEX_PATH = join(CACHE_DIR, "index.json");
const CACHE_LOGS_DIR = join(CACHE_DIR, "logs");
const CACHE_VERSION = 1;

function sha256(data: string | Buffer): string {
  return createHash("sha256").update(data).digest("hex").slice(0, 16);
}

function loadCache(): CacheIndex {
  try {
    if (existsSync(CACHE_INDEX_PATH)) {
      const data = JSON.parse(readFileSync(CACHE_INDEX_PATH, "utf-8"));
      if (data.version === CACHE_VERSION) {
        return data;
      }
      console.log("Cache version mismatch, starting fresh");
    }
  } catch (e) {
    console.log("Cache corrupted, starting fresh");
  }
  return { version: CACHE_VERSION, entries: {} };
}

function saveCache(cache: CacheIndex): void {
  mkdirSync(CACHE_DIR, { recursive: true });
  writeFileSync(CACHE_INDEX_PATH, JSON.stringify(cache, null, 2));
}

function getBinaryHash(binName: string): string {
  try {
    const binaryPath = execSync(`which ${binName}`, { encoding: "utf-8" }).trim();
    const binaryContent = readFileSync(binaryPath);
    return sha256(binaryContent);
  } catch (e) {
    // Fallback to version string if we can't read the binary
    try {
      const version = execSync(`${binName} --version 2>/dev/null || echo "unknown"`, { encoding: "utf-8" }).trim();
      return sha256(version);
    } catch {
      return "unknown";
    }
  }
}

function getMcpHarnessHash(): string {
  const mcpHarnessPath = join(import.meta.dirname, "../../mcp-harness/dist/index.js");
  try {
    if (existsSync(mcpHarnessPath)) {
      return sha256(readFileSync(mcpHarnessPath));
    }
  } catch (e) {
    // Ignore
  }
  return "no-mcp-harness";
}

function computeCacheKey(pair: TestPair, binaryHashes: Map<string, string>, mcpHarnessHash: string): { key: string; inputs: CacheInputs } {
  // Hash scenario content (name + prompt/turns + setup + validate)
  const scenarioContent = stringify({
    name: pair.scenario.name,
    prompt: pair.scenario.prompt,
    turns: pair.scenario.turns,
    setup: pair.scenario.setup,
    validate: pair.scenario.validate,
  });
  const scenarioHash = sha256(scenarioContent);

  // Model key
  const modelKey = `${pair.model.provider}/${pair.model.model}`;

  // Hash runner config
  const runnerContent = JSON.stringify({
    name: pair.runner.name,
    type: pair.runner.type,
    extensions: pair.runner.extensions ?? [],
    stdio: pair.runner.stdio ?? [],
  });
  const runnerHash = sha256(runnerContent);

  // Binary hash (cached per binary name)
  const binaryHash = binaryHashes.get(pair.runner.bin) ?? "unknown";

  const inputs: CacheInputs = {
    scenarioHash,
    modelKey,
    runnerHash,
    binaryHash,
    mcpHarnessHash,
  };

  // Combine all into single key
  const key = sha256(scenarioHash + modelKey + runnerHash + binaryHash + mcpHarnessHash);

  return { key, inputs };
}

function getCachedResult(
  cache: CacheIndex,
  cacheKey: string,
  pair: TestPair,
  logsDir: string
): TestResultWithLog | null {
  const entry = cache.entries[cacheKey];
  if (!entry) return null;

  // Verify the cached log file exists
  const cachedLogPath = join(CACHE_LOGS_DIR, entry.logFile);
  if (!existsSync(cachedLogPath)) {
    console.log(`  Cache log missing, will re-run`);
    delete cache.entries[cacheKey];
    return null;
  }

  // Copy cached log to current logs directory
  const testId = `${pair.scenario.name}_${pair.model.name}_${pair.runner.name}`.replace(/[\/\\:]/g, "_");
  const logFile = join(logsDir, `${testId}_cached.log`);
  mkdirSync(logsDir, { recursive: true });
  copyFileSync(cachedLogPath, logFile);

  // Reconstruct result
  const config = {
    provider: pair.model.provider,
    model: pair.model.model,
    extensions: pair.runner.extensions,
    stdio: pair.runner.stdio,
  };

  const run: TestRun = {
    scenario: pair.scenario,
    config,
    workdir: "", // Not relevant for cached results
    startTime: new Date(entry.timestamp),
    endTime: new Date(new Date(entry.timestamp).getTime() + entry.result.duration),
    status: entry.result.status,
    errors: entry.result.errors,
  };

  return {
    run,
    validations: entry.result.validations,
    logFile,
    runnerName: pair.runner.name,
    toolCalls: entry.result.toolCalls,
    turns: entry.result.turns,
    cached: true,
  };
}

function storeCacheResult(
  cache: CacheIndex,
  cacheKey: string,
  inputs: CacheInputs,
  result: TestResultWithLog
): void {
  // Copy log to cache directory
  const logFileName = `${cacheKey}.log`;
  const cachedLogPath = join(CACHE_LOGS_DIR, logFileName);
  mkdirSync(CACHE_LOGS_DIR, { recursive: true });
  
  try {
    copyFileSync(result.logFile, cachedLogPath);
  } catch (e) {
    console.log(`  Warning: Could not cache log file`);
    return;
  }

  cache.entries[cacheKey] = {
    timestamp: new Date().toISOString(),
    inputs,
    result: {
      status: result.run.status as "passed" | "failed",
      validations: result.validations,
      duration: result.run.endTime && result.run.startTime
        ? result.run.endTime.getTime() - result.run.startTime.getTime()
        : 0,
      toolCalls: result.toolCalls,
      turns: result.turns,
      errors: result.run.errors,
    },
    logFile: logFileName,
  };

  saveCache(cache);
}

function clearCache(): void {
  if (existsSync(CACHE_DIR)) {
    rmSync(CACHE_DIR, { recursive: true, force: true });
    console.log("Cache cleared");
  } else {
    console.log("No cache to clear");
  }
}

// =============================================================================
// Goose Runner
// =============================================================================

const PLATFORM_EXTENSIONS = new Set([
  "todo", "skills", "code_execution", "extensionmanager", 
  "chatrecall", "apps", "imagegenerator"
]);

// Isolated goose config directory
const GOOSE_ROOT = join(import.meta.dirname, "../.goose-root");
const GOOSE_CONFIG_DIR = join(GOOSE_ROOT, "config");

function generateGooseConfig(model: ModelConfig, runner: RunnerConfig): object {
  const extensions: Record<string, object> = {};

  // Add extensions (detect platform vs builtin)
  for (const ext of runner.extensions ?? []) {
    if (PLATFORM_EXTENSIONS.has(ext)) {
      extensions[ext] = {
        enabled: true,
        type: "platform",
        name: ext,
        bundled: true,
      };
    } else {
      extensions[ext] = {
        enabled: true,
        type: "builtin",
        name: ext,
        timeout: 300,
        bundled: true,
      };
    }
  }

  // Add stdio MCP servers
  for (const extCmd of runner.stdio ?? []) {
    const parts = extCmd.split(" ");
    const cmd = parts[0];
    const args = parts.slice(1);
    const name = basename(args[args.length - 1] || cmd).replace(/\.[^.]+$/, "");

    extensions[name] = {
      enabled: true,
      type: "stdio",
      name,
      cmd,
      args,
      timeout: 300,
    };
  }

  return {
    extensions,
    GOOSE_PROVIDER: model.provider,
    GOOSE_MODEL: model.model,
    GOOSE_TELEMETRY_ENABLED: false,
  };
}

async function runGooseAgent(
  model: ModelConfig,
  runner: RunnerConfig,
  prompt: string,
  workdir: string,
  sessionName?: string,  // If provided, use/continue this session
  resume: boolean = false  // If true, resume existing session (for turn 2+)
): Promise<string> {
  const promptFile = join(workdir, ".goose-prompt.txt");
  writeFileSync(promptFile, prompt);

  // Write goose config
  mkdirSync(GOOSE_CONFIG_DIR, { recursive: true });
  const gooseConfig = generateGooseConfig(model, runner);
  writeFileSync(join(GOOSE_CONFIG_DIR, "config.yaml"), stringify(gooseConfig));

  let cmd: string;
  if (sessionName) {
    if (resume) {
      cmd = `${runner.bin} run -i "${promptFile}" --name "${sessionName}" --resume`;
      console.log(`  Running: ${runner.bin} run -i <prompt> --name "${sessionName}" --resume`);
    } else {
      // First turn: create new session with this name
      cmd = `${runner.bin} run -i "${promptFile}" --name "${sessionName}"`;
      console.log(`  Running: ${runner.bin} run -i <prompt> --name "${sessionName}"`);
    }
  } else {
    cmd = `${runner.bin} run -i "${promptFile}" --no-session`;
    console.log(`  Running: ${runner.bin} run -i <prompt> --no-session`);
  }

  const output = execSync(cmd, {
    cwd: workdir,
    env: {
      ...process.env,
      GOOSE_PATH_ROOT: GOOSE_ROOT,
      MCP_HARNESS_LOG: join(workdir, "tool-calls.log"),
    },
    timeout: 5 * 60 * 1000,
    encoding: "utf-8",
  });

  return output;
}

// =============================================================================
// OpenCode Runner
// =============================================================================

// Isolated opencode config directory
const OPENCODE_ROOT = join(import.meta.dirname, "../.opencode-root");

function generateOpenCodeConfig(model: ModelConfig, runner: RunnerConfig, workdir: string): object {
  const mcp: Record<string, object> = {};

  // Add stdio MCP servers
  for (const extCmd of runner.stdio ?? []) {
    const parts = extCmd.split(" ");
    const cmd = parts[0];
    const args = parts.slice(1);
    const name = basename(args[args.length - 1] || cmd).replace(/\.[^.]+$/, "");

    mcp[name] = {
      type: "local",
      command: [cmd, ...args],
      enabled: true,
      environment: {
        MCP_HARNESS_LOG: join(workdir, "tool-calls.log"),
      },
    };
  }

  const config: Record<string, any> = {
    $schema: "https://opencode.ai/config.json",
    mcp,
  };

  // Handle ollama as a custom provider (OpenCode doesn't have built-in ollama support)
  if (model.provider === "ollama") {
    config.model = `ollama/${model.model}`;
    config.provider = {
      ollama: {
        npm: "@ai-sdk/openai-compatible",
        name: "Ollama (local)",
        options: {
          baseURL: "http://localhost:11434/v1",
        },
        models: {
          [model.model]: {
            name: model.name,
          },
        },
      },
    };
  } else {
    // Standard providers (anthropic, openai, etc.)
    config.model = `${model.provider}/${model.model}`;
  }

  return config;
}

async function runOpenCodeAgent(
  model: ModelConfig,
  runner: RunnerConfig,
  prompt: string,
  workdir: string,
  resume: boolean = false
): Promise<string> {
  // Write opencode.json config to workdir
  const openCodeConfig = generateOpenCodeConfig(model, runner, workdir);
  writeFileSync(join(workdir, "opencode.json"), JSON.stringify(openCodeConfig, null, 2));

  // Write prompt to file (use cat to avoid shell escaping issues)
  const promptFile = join(workdir, ".opencode-prompt.txt");
  writeFileSync(promptFile, prompt);

  // Ensure isolated config directory exists
  mkdirSync(OPENCODE_ROOT, { recursive: true });

  // Use --continue on turn 2+ to continue last session
  const continueFlag = resume ? "--continue " : "";
  const cmd = `${runner.bin} run ${continueFlag}"$(cat "${promptFile}")"`;
  console.log(`  Running: ${runner.bin} run ${continueFlag}"<prompt>"`);

  const output = execSync(cmd, {
    cwd: workdir,
    env: {
      ...process.env,
      XDG_CONFIG_HOME: OPENCODE_ROOT,
      XDG_DATA_HOME: OPENCODE_ROOT,
    },
    timeout: 5 * 60 * 1000,
    encoding: "utf-8",
    shell: "/bin/bash",
  });

  return output;
}


// =============================================================================
// Pi Runner
// =============================================================================

// Pi takes --provider and --model as CLI arguments
// MCP support via pi-mcp-adapter: `pi install npm:pi-mcp-adapter`

// Isolated Pi config directory (like Goose/OpenCode)
const PI_CONFIG_DIR = join(import.meta.dirname, "../.pi-root");

// User's real Pi config (for copying auth.json)
const PI_USER_CONFIG = join(homedir(), ".pi", "agent");

/**
 * Generate models.json for Pi with the test model.
 * For ollama models, we need to define them since Pi doesn't have built-in ollama support.
 */
function generatePiModelsConfig(model: ModelConfig): object {
  // Only generate config for ollama provider (others are built-in)
  if (model.provider !== "ollama") {
    return { providers: {} };
  }

  return {
    providers: {
      ollama: {
        baseUrl: "http://localhost:11434/v1",
        api: "openai-completions",
        apiKey: "ollama",  // Ollama doesn't need a real key
        models: [
          {
            id: model.model,
            name: model.name,
            reasoning: false,
            input: ["text"],
            contextWindow: 128000,
            maxTokens: 32768,
            compat: {
              supportsUsageInStreaming: false,
              maxTokensField: "max_tokens",
              supportsDeveloperRole: false
            }
          }
        ]
      }
    }
  };
}

async function runPiAgent(
  model: ModelConfig,
  runner: RunnerConfig,
  prompt: string,
  workdir: string,
  sessionName?: string,  // If provided, use/continue this session (for multi-turn)
  resume: boolean = false  // If true, continue existing session (for turn 2+)
): Promise<string> {
  // Write prompt to file (use cat to avoid shell escaping issues)
  const promptFile = join(workdir, ".pi-prompt.txt");
  writeFileSync(promptFile, prompt);

  // Set up isolated Pi config directory
  mkdirSync(PI_CONFIG_DIR, { recursive: true });

  // Generate models.json with the test model (for ollama)
  const modelsConfig = generatePiModelsConfig(model);
  writeFileSync(join(PI_CONFIG_DIR, "models.json"), JSON.stringify(modelsConfig, null, 2));

  // Copy auth.json from user's config (for API keys)
  const userAuthPath = join(PI_USER_CONFIG, "auth.json");
  if (existsSync(userAuthPath)) {
    copyFileSync(userAuthPath, join(PI_CONFIG_DIR, "auth.json"));
  }

  // Copy settings.json from user's config (for installed packages like pi-mcp-adapter)
  const userSettingsPath = join(PI_USER_CONFIG, "settings.json");
  if (existsSync(userSettingsPath)) {
    copyFileSync(userSettingsPath, join(PI_CONFIG_DIR, "settings.json"));
  }

  // If runner has stdio MCP servers, write .pi/mcp.json to the workdir (project config)
  // pi-mcp-adapter checks for .pi/mcp.json in cwd, which overrides global config
  let hasMcp = false;
  if (runner.stdio?.length) {
    const mcpConfig: {
      mcpServers: Record<string, {
        command: string;
        args: string[];
        lifecycle: string;
        env: Record<string, string>;
      }>;
      settings: { toolPrefix: string };
    } = {
      mcpServers: {},
      settings: {
        toolPrefix: "none"   // No prefix - use raw tool names
      }
      // Proxy mode: LLM uses mcp({ search: "..." }) to discover tools on-demand
      // This scales better with many MCP tools vs directTools which burns context
    };

    // Add each stdio server from runner config
    runner.stdio.forEach((extCmd, i) => {
      const parts = extCmd.split(" ");
      const serverName = `harness${i > 0 ? i : ''}`;
      mcpConfig.mcpServers[serverName] = {
        command: parts[0],
        args: parts.slice(1),
        lifecycle: "eager",  // Connect at startup for tests
        env: {
          MCP_HARNESS_LOG: join(workdir, "tool-calls.log")
        }
      };
    });

    // Write .pi/mcp.json to workdir (project-local config that pi-mcp-adapter finds)
    const piConfigDir = join(workdir, ".pi");
    mkdirSync(piConfigDir, { recursive: true });
    writeFileSync(join(piConfigDir, "mcp.json"), JSON.stringify(mcpConfig, null, 2));
    hasMcp = true;
  }

  // Build base command with provider/model
  // -p = non-interactive (print mode)
  let cmd = `${runner.bin} -p --provider ${model.provider} --model "${model.model}"`;

  // Session handling for multi-turn
  if (sessionName) {
    const sessionPath = join(workdir, `.pi-session-${sessionName}.jsonl`);
    if (resume) {
      // Turn 2+: continue the existing session
      cmd += ` --continue --session "${sessionPath}"`;
    } else {
      // Turn 1: create a new session file
      cmd += ` --session "${sessionPath}"`;
    }
  } else {
    // Single-turn: don't save session
    cmd += ` --no-session`;
  }

  cmd += ` "$(cat "${promptFile}")"`;

  // Build log message
  const sessionInfo = sessionName 
    ? (resume ? ` --continue --session <session>` : ` --session <session>`)
    : ` --no-session`;
  console.log(`  Running: ${runner.bin} -p${sessionInfo} --provider ${model.provider} --model "${model.model}"${hasMcp ? ' (mcp)' : ''} "<prompt>"`);

  const output = execSync(cmd, {
    cwd: workdir,
    env: {
      ...process.env,
      PI_CODING_AGENT_DIR: PI_CONFIG_DIR,  // Use isolated config dir
      MCP_HARNESS_LOG: join(workdir, "tool-calls.log"),
    },
    timeout: 5 * 60 * 1000,
    encoding: "utf-8",
    shell: "/bin/bash",
  });

  return output;
}

// =============================================================================
// Unified Runner
// =============================================================================

interface AgentResult {
  output: string;
  sessionId?: string;  // For multi-turn (goose, pi)
}

async function runAgent(
  model: ModelConfig,
  runner: RunnerConfig,
  prompt: string,
  workdir: string,
  sessionId?: string,  // For multi-turn (goose, pi)
  resume: boolean = false  // For multi-turn: true on turn 2+
): Promise<AgentResult> {
  if (runner.type === "opencode") {
    const output = await runOpenCodeAgent(model, runner, prompt, workdir, resume);
    return { output };
  }
  if (runner.type === "pi") {
    const output = await runPiAgent(model, runner, prompt, workdir, sessionId, resume);
    return { output, sessionId };
  }
  const output = await runGooseAgent(model, runner, prompt, workdir, sessionId, resume);
  return { output, sessionId };
}

// =============================================================================
// Scenario & Config Loading
// =============================================================================

function loadScenario(path: string): Scenario {
  const content = readFileSync(path, "utf-8");
  return parse(content) as Scenario;
}

function loadAllScenarios(dir: string): Scenario[] {
  const files = readdirSync(dir).filter((f) => f.endsWith(".yaml"));
  return files.map((f) => loadScenario(join(dir, f)));
}

function loadConfig(configPath: string): SuiteConfig {
  const content = readFileSync(configPath, "utf-8");
  const config = parse(content) as SuiteConfig;
  const configDir = join(configPath, "..");

  // Resolve relative paths in stdio for all runners
  for (const runner of config.runners) {
    if (runner.stdio) {
      runner.stdio = runner.stdio.map((ext) => {
        const parts = ext.split(" ");
        const cmd = parts[0];
        const args = parts.slice(1).map((arg) => {
          if (!arg.startsWith("/") && (arg.includes("/") || arg.startsWith("."))) {
            return join(configDir, arg);
          }
          return arg;
        });
        return [cmd, ...args].join(" ");
      });
    }
  }

  return config;
}

function setupWorkdir(scenario: Scenario, workdir: string): void {
  rmSync(workdir, { recursive: true, force: true });
  mkdirSync(workdir, { recursive: true });

  if (scenario.setup) {
    for (const [path, content] of Object.entries(scenario.setup)) {
      const fullPath = join(workdir, path);
      mkdirSync(join(fullPath, ".."), { recursive: true });
      writeFileSync(fullPath, content);
    }
  }
}

// =============================================================================
// Log Metrics Parsing
// =============================================================================

function parseLogMetrics(logContent: string, workdir?: string): { toolCalls: number; turns: number } {
  // First, try to read tool-calls.log from MCP harness (most accurate)
  let mcpToolCalls = 0;
  if (workdir) {
    try {
      const toolCallsLog = readFileSync(join(workdir, "tool-calls.log"), "utf-8");
      // Each line is a JSON object representing one tool call
      mcpToolCalls = toolCallsLog.trim().split("\n").filter(line => line.trim()).length;
    } catch (e) {
      // tool-calls.log doesn't exist, fall back to log parsing
    }
  }

  // Goose format: ─── tool_name | extension ───
  const gooseToolCalls = (logContent.match(/─── .+ \| .+ ───/g) || []).length;
  
  // OpenCode format: TURN N
  const opencodeTurns = (logContent.match(/^TURN \d+$/gm) || []).length;
  
  // Total tool calls = MCP harness calls + Goose built-in tool calls
  const toolCalls = mcpToolCalls + gooseToolCalls;

  // For OpenCode, use explicit TURN markers
  const turns = opencodeTurns > 0 ? opencodeTurns : Math.ceil(toolCalls / 3); // Estimate ~3 tool calls per turn
  
  return { toolCalls, turns };
}

// =============================================================================
// Test Execution
// =============================================================================

function buildTestPairs(config: SuiteConfig, scenarios: Scenario[]): TestPair[] {
  const modelsByName = new Map(config.models.map((m) => [m.name, m]));
  const runnersByName = new Map(config.runners.map((r) => [r.name, r]));
  const scenariosByName = new Map(scenarios.map((s) => [s.name, s]));

  const pairs: TestPair[] = [];

  if (config.matrix?.length) {
    for (const entry of config.matrix) {
      // Validate scenario name
      const scenario = scenariosByName.get(entry.scenario);
      if (!scenario) {
        throw new Error(`Unknown scenario "${entry.scenario}" in matrix. Available: ${[...scenariosByName.keys()].join(", ")}`);
      }

      // Validate model names
      if (entry.models) {
        for (const name of entry.models) {
          if (!modelsByName.has(name)) {
            throw new Error(`Unknown model "${name}" in matrix entry for scenario "${entry.scenario}". Available: ${[...modelsByName.keys()].join(", ")}`);
          }
        }
      }

      // Validate runner names
      if (entry.runners) {
        for (const name of entry.runners) {
          if (!runnersByName.has(name)) {
            throw new Error(`Unknown runner "${name}" in matrix entry for scenario "${entry.scenario}". Available: ${[...runnersByName.keys()].join(", ")}`);
          }
        }
      }

      const models = entry.models
        ? entry.models.map((n) => modelsByName.get(n)).filter(Boolean) as ModelConfig[]
        : config.models;

      const runners = entry.runners
        ? entry.runners.map((n) => runnersByName.get(n)).filter(Boolean) as RunnerConfig[]
        : config.runners;

      for (const model of models) {
        for (const runner of runners) {
          pairs.push({ scenario, model, runner });
        }
      }
    }
    return pairs;
  }

  // No matrix: all scenarios × all models × all runners
  for (const scenario of scenarios) {
    for (const model of config.models) {
      for (const runner of config.runners) {
        pairs.push({ scenario, model, runner });
      }
    }
  }
  return pairs;
}

function scoreResult(result: TestResultWithLog): number {
  if (result.run.status === "failed" && result.run.errors?.length) {
    return -1;
  }
  const passedCount = result.validations.filter((v) => v.passed).length;
  const statusBonus = result.run.status === "passed" ? 1000 : 0;
  return statusBonus + passedCount;
}

async function runScenario(
  pair: TestPair,
  baseWorkdir: string,
  logsDir: string,
  attempt: number = 1
): Promise<TestResultWithLog> {
  const { scenario, model, runner } = pair;
  const testId = `${scenario.name}_${model.name}_${runner.name}`.replace(/[\/\\:]/g, "_");
  const workdir = join(baseWorkdir, testId);
  const logFile = join(logsDir, `${testId}_attempt${attempt}.log`);

  console.log(`\n▶ ${scenario.name} [${model.provider}/${model.model}] (${runner.name})`);

  setupWorkdir(scenario, workdir);
  mkdirSync(logsDir, { recursive: true });

  // Create a minimal config for TestRun compatibility
  const config = {
    provider: model.provider,
    model: model.model,
    extensions: runner.extensions,
    stdio: runner.stdio,
  };

  const run: TestRun = {
    scenario,
    config,
    workdir,
    startTime: new Date(),
    status: "running",
  };

  // Determine if this is a multi-turn or single-turn scenario
  const turns = scenario.turns ?? [
    { prompt: scenario.prompt!, validate: scenario.validate ?? [] }
  ];
  const isMultiTurn = turns.length > 1;

  // For goose/pi: generate session ID upfront
  // For opencode: capture session ID from first turn's output
  let sessionId: string | undefined = isMultiTurn && (runner.type === "goose" || runner.type === "pi")
    ? `test_${testId}_${Date.now()}`
    : undefined;

  let output = "";
  const allValidations: Array<{ rule: any; passed: boolean; message?: string }> = [];

  try {
    for (let turnIndex = 0; turnIndex < turns.length; turnIndex++) {
      const turn = turns[turnIndex];
      const turnLabel = isMultiTurn ? ` [turn ${turnIndex + 1}/${turns.length}]` : "";
      console.log(`  Running${turnLabel}...`);

      // Run the agent (with session for multi-turn)
      const resume = turnIndex > 0;  // Resume session on turn 2+
      const result = await runAgent(model, runner, turn.prompt, workdir, sessionId, resume);
      
      // Capture session ID from first turn (for opencode)
      if (turnIndex === 0 && result.sessionId) {
        sessionId = result.sessionId;
      }
      
      output += `\n${'='.repeat(60)}\nTURN ${turnIndex + 1}\n${'='.repeat(60)}\n${result.output}`;

      // Validate this turn
      const turnValidations = validateAll(turn.validate, workdir);
      for (const v of turnValidations) {
        allValidations.push({
          rule: v.rule,
          passed: v.result.passed,
          message: v.result.message,
        });
      }

      // If any validation failed, stop early
      const turnPassed = turnValidations.every((v) => v.result.passed);
      if (!turnPassed) {
        console.log(`  Turn ${turnIndex + 1} failed validation`);
        break;
      }
    }

    run.endTime = new Date();
    const allPassed = allValidations.every((v) => v.passed);

    writeFileSync(logFile, output);

    const metrics = parseLogMetrics(output, workdir);
    return {
      run: { ...run, status: allPassed ? "passed" : "failed" },
      validations: allValidations,
      logFile,
      runnerName: runner.name,
      toolCalls: metrics.toolCalls,
      turns: metrics.turns,
    };
  } catch (err) {
    const errorOutput = output + "\n\nERROR:\n" + String(err);
    writeFileSync(logFile, errorOutput);

    return {
      run: {
        ...run,
        status: "failed",
        endTime: new Date(),
        errors: [String(err)],
      },
      validations: allValidations,
      logFile,
      runnerName: runner.name,
      toolCalls: parseLogMetrics(errorOutput, workdir).toolCalls,
      turns: parseLogMetrics(errorOutput, workdir).turns,
    };
  }
}

// =============================================================================
// Reporting
// =============================================================================

function pairKey(pair: TestPair): string {
  return `${pair.model.name}::${pair.runner.name}`;
}

function resultKey(result: TestResultWithLog): string {
  return `${result.run.config.provider}/${result.run.config.model}::${result.runnerName}`;
}

interface ReportOptions {
  isRunning?: boolean;
  allPairs?: TestPair[];
}

function generateHtmlReport(
  results: TestResultWithLog[],
  outputPath: string,
  options: ReportOptions = {}
): void {
  const { isRunning = false, allPairs = [] } = options;

  // Read and embed gym.png as base64
  const rootDir = join(outputPath, "..");
  let gymBase64 = "";
  try {
    const gymPath = join(rootDir, "gym.png");
    gymBase64 = readFileSync(gymPath).toString("base64");
  } catch (e) {
    // gym.png not found, will use external reference
  }

  // Collect all logs for embedding
  const logsData: Record<string, string> = {};
  for (const r of results) {
    if (r.logFile) {
      try {
        logsData[basename(r.logFile)] = readFileSync(r.logFile, "utf-8");
      } catch (e) { /* ignore missing logs */ }
    }
  }

  // Calculate max duration for scaling bars
  const maxDuration = Math.max(...results.map(r => {
    if (!r.run.endTime || !r.run.startTime) return 0;
    return (r.run.endTime.getTime() - r.run.startTime.getTime()) / 1000;
  }), 1);
  
  const maxToolCalls = Math.max(...results.map(r => r.toolCalls || 0), 1);

  // Get all scenarios (columns)
  const scenarios = allPairs.length
    ? [...new Set(allPairs.map((p) => p.scenario.name))]
    : [...new Set(results.map((r) => r.run.scenario.name))];

  // Rows are model × runner combinations
  const rowKeys = allPairs.length
    ? [...new Set(allPairs.map(pairKey))]
    : [...new Set(results.map((r) => `${r.run.config.provider}/${r.run.config.model}::${r.runnerName}`))];

  // Map row key -> pair info
  const rowsByKey = new Map<string, { model: ModelConfig; runner: RunnerConfig }>();
  for (const pair of allPairs) {
    rowsByKey.set(pairKey(pair), { model: pair.model, runner: pair.runner });
  }

  // Group rows by model for rowspan display
  const modelKey = (m: ModelConfig) => `${m.provider}/${m.model}`;
  const modelGroups = new Map<string, string[]>();  // modelKey -> rowKeys[]
  for (const key of rowKeys) {
    const row = rowsByKey.get(key);
    if (!row) continue;
    const mk = modelKey(row.model);
    if (!modelGroups.has(mk)) modelGroups.set(mk, []);
    modelGroups.get(mk)!.push(key);
  }

  // Build set of valid (scenario, rowKey) combinations from the matrix
  const validCells = new Set<string>();
  for (const pair of allPairs) {
    validCells.add(`${pair.scenario.name}::${pairKey(pair)}`);
  }

  const getResult = (scenario: string, rowKey: string) => {
    const [modelPart, runnerName] = rowKey.split("::");
    return results.find(
      (r) =>
        r.run.scenario.name === scenario &&
        `${r.run.config.provider}/${r.run.config.model}` === `${rowsByKey.get(rowKey)?.model.provider}/${rowsByKey.get(rowKey)?.model.model}` &&
        r.runnerName === runnerName
    );
  };

  const passed = results.filter((r) => r.run.status === "passed").length;
  const failed = results.filter((r) => r.run.status === "failed").length;
  const total = allPairs.length || results.length;
  const pending = total - results.length;

  const runnerNames = [...new Set(allPairs.map((p) => p.runner.name))];

  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  
  <title>${isRunning ? "Running..." : "Results"} - Agent Gym Workout</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0d1117; color: #c9d1d9; padding: 2rem; }
    h1 { color: #58a6ff; margin-bottom: 0.5rem; }
    .summary { color: #8b949e; margin-bottom: 2rem; font-size: 1.1rem; }
    .summary .passed { color: #3fb950; }
    .summary .failed { color: #f85149; }
    table { width: 100%; border-collapse: collapse; background: #161b22; border-radius: 8px; overflow: hidden; }
    th, td { padding: 1rem; text-align: left; border-bottom: 1px solid #30363d; }
    th { background: #21262d; color: #58a6ff; font-weight: 600; }
    th:first-child { position: sticky; left: 0; background: #21262d; z-index: 1; }
    td:first-child { position: sticky; left: 0; background: #161b22; font-weight: 500; }
    .cell { display: flex; align-items: center; gap: 0.5rem; }
    .status { width: 24px; height: 24px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 14px; }
    .status.passed { background: #238636; }
    .status.failed { background: #da3633; }
    .status.pending { background: #6e7681; }
    .status.na { background: transparent; border: 1px dashed #30363d; color: #484f58; }
    .status.running { background: #9e6a03; animation: pulse 1.5s infinite; }
    @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
    .cached-badge { background: #388bfd33; color: #58a6ff; font-size: 0.65rem; padding: 0.1rem 0.3rem; border-radius: 3px; margin-left: 0.25rem; }
    .duration { color: #8b949e; font-size: 0.85rem; }
    .log-link { color: #58a6ff; font-size: 0.75rem; text-decoration: none; }
    .log-link:hover { text-decoration: underline; }
    .duration-bar { height: 4px; background: #30363d; border-radius: 2px; margin-top: 4px; overflow: hidden; }
    .duration-bar-fill { height: 100%; background: #58a6ff; border-radius: 2px; }
    .tool-bar { height: 4px; background: #30363d; border-radius: 2px; margin-top: 2px; overflow: hidden; }
    .tool-bar-fill { height: 100%; background: #d29922; border-radius: 2px; }
    .metrics { display: flex; gap: 0.5rem; font-size: 0.7rem; color: #8b949e; margin-top: 2px; }
    .metrics .tool-calls { color: #d29922; }
    .metrics .turns { color: #a371f7; }
    .details { font-size: 0.8rem; max-width: 300px; }
    .validation { display: flex; align-items: center; gap: 0.25rem; margin-top: 0.25rem; }
    .validation.pass { color: #3fb950; }
    .validation.fail { color: #f85149; }
    .validation-icon { font-size: 0.7rem; }
    .row-header { font-size: 0.75rem; }
    .row-header .model { display: block; color: #c9d1d9; font-weight: 600; }
    .row-header .runner { color: #58a6ff; }
    .row-header .runner-type { color: #8b949e; font-size: 0.7rem; }
    .model-cell { vertical-align: middle; background: #1c2128 !important; border-right: 2px solid #30363d; }
    .model-group-start { border-top: 2px solid #30363d; }
    .runner-info { color: #6e7681; font-size: 0.85rem; margin-bottom: 1.5rem; }
    .runner-info code { background: #21262d; padding: 0.2rem 0.4rem; border-radius: 4px; font-family: monospace; }
    .timestamp { color: #6e7681; font-size: 0.9rem; margin-top: 2rem; }
    .header { display: flex; align-items: center; gap: 1rem; margin-bottom: 0.5rem; }
    .header img { height: 48px; width: auto; }
    .header h1 { margin: 0; }
    .download-btn { background: #238636; color: white; border: none; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; font-size: 0.9rem; margin-left: auto; }
    .download-btn:hover { background: #2ea043; }
    .modal { display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.8); z-index: 100; }
    .modal.open { display: flex; align-items: center; justify-content: center; }
    .modal-content { background: #161b22; border: 1px solid #30363d; border-radius: 8px; width: 90%; max-width: 1200px; max-height: 90%; display: flex; flex-direction: column; }
    .modal-header { display: flex; align-items: center; justify-content: space-between; padding: 1rem; border-bottom: 1px solid #30363d; }
    .modal-header h2 { margin: 0; color: #58a6ff; font-size: 1rem; }
    .modal-close { background: none; border: none; color: #8b949e; font-size: 1.5rem; cursor: pointer; }
    .modal-close:hover { color: #c9d1d9; }
    .modal-body { padding: 1rem; overflow: auto; flex: 1; }
    .modal-body pre { margin: 0; white-space: pre-wrap; word-wrap: break-word; font-family: monospace; font-size: 0.85rem; color: #c9d1d9; }
  </style>
</head>
<body data-running="${isRunning}">
  <div class="header"><img src="${gymBase64 ? `data:image/png;base64,${gymBase64}` : 'gym.png'}" alt="Agent Gym"><h1>Agent Gym Workout${isRunning ? " (Running...)" : ""}</h1>${!isRunning ? '<button class="download-btn" onclick="downloadReport()">📥 Download Full Report</button>' : ''}</div>
  <p class="summary">
    <span class="passed">${passed} passed</span> / 
    <span class="failed">${failed} failed</span>${pending > 0 ? ` / <span style="color:#9e6a03">${pending} pending</span>` : ""} / 
    ${total} total
  </p>
  <p class="runner-info">Agent Configurations: ${runnerNames.map(n => `<code>${n}</code>`).join(", ")}</p>
  
  <table>
    <thead>
      <tr>
        <th>Model</th>
        <th>Agent Configuration</th>
        ${scenarios.map((s) => `<th>${s}</th>`).join("")}
      </tr>
    </thead>
    <tbody>
      ${[...modelGroups.entries()].map(([mk, keys]) => {
        return keys.map((key, idx) => {
          const row = rowsByKey.get(key);
          if (!row) return "";
          const { model, runner } = row;
          const isFirst = idx === 0;
          const rowspan = keys.length;
          return `
          <tr class="${isFirst ? 'model-group-start' : ''}">
            ${isFirst ? `<td class="model-cell" rowspan="${rowspan}"><div class="row-header">
              <span class="model">${model.provider}/${model.model}</span>
            </div></td>` : ''}
            <td><div class="row-header">
              <span class="runner">${runner.name}</span>
              <span class="runner-type">(${runner.type})</span>
            </div></td>
            ${scenarios.map((scenario) => {
              const r = getResult(scenario, key);
              if (!r) {
                // Check if this combination is in the matrix
                const cellKey = `${scenario}::${key}`;
                const isInMatrix = validCells.has(cellKey);
                if (!isInMatrix) return `<td><div class="cell"><span class="status na">—</span></div></td>`;
                return `<td><div class="cell"><span class="status pending">⋯</span></div></td>`;
              }
              if (r.run.status === "running") {
                return `<td><div class="cell"><span class="status running">...</span></div></td>`;
              }
              const duration = r.run.endTime
                ? ((r.run.endTime.getTime() - r.run.startTime.getTime()) / 1000).toFixed(1)
                : "-";
              const logPath = r.logFile ? `logs/${basename(r.logFile)}` : "";
              const validationHtml = r.validations.map((v) => {
                const icon = v.passed ? "✓" : "✗";
                const cls = v.passed ? "pass" : "fail";
                const ruleLabel = (v.rule as any).name 
                  ? (v.rule as any).name
                  : v.rule.type === "tool_called" 
                    ? `tool_called: ${(v.rule as any).tool}`
                    : v.rule.type + (("path" in v.rule) ? `: ${(v.rule as any).path}` : "");
                return `<div class="validation ${cls}"><span class="validation-icon">${icon}</span> ${ruleLabel}</div>`;
              }).join("");
              return `<td>
                <div class="cell">
                  <span class="status ${r.run.status}">${r.run.status === "passed" ? "✓" : "✗"}</span>
                  ${r.cached ? '<span class="cached-badge">cached</span>' : ''}
                  <span class="duration">${duration}s</span>
                  ${logPath ? `<a class="log-link" href="${logPath}" onclick="event.preventDefault();showLog('${basename(r.logFile!)}')">log</a>` : ""}
                </div>
                <div class="duration-bar"><div class="duration-bar-fill" style="width: ${Math.round((parseFloat(duration) / maxDuration) * 100)}%"></div></div>
                <div class="tool-bar"><div class="tool-bar-fill" style="width: ${Math.round(((r.toolCalls || 0) / maxToolCalls) * 100)}%"></div></div>
                <div class="metrics">
                  <span class="tool-calls" title="Tool calls">🔧 ${r.toolCalls || 0}</span>
                  <span class="turns" title="Turns">↻ ${r.turns || 0}</span>
                </div>
                <div class="details">${validationHtml}</div>
              </td>`;
            }).join("")}
          </tr>`;
        }).join("");
      }).join("")}
    </tbody>
  </table>
  
  <p class="timestamp">Generated: ${new Date().toISOString()}</p>

  <!-- Log Modal -->
  <div id="logModal" class="modal" onclick="if(event.target===this)closeModal()">
    <div class="modal-content">
      <div class="modal-header">
        <h2 id="modalTitle">Log</h2>
        <button class="modal-close" onclick="closeModal()">&times;</button>
      </div>
      <div class="modal-body">
        <pre id="modalLog"></pre>
      </div>
    </div>
  </div>

  <script>
    const LOGS = ${JSON.stringify(logsData)};
    
    function showLog(logName) {
      const log = LOGS[logName];
      if (!log) return;
      document.getElementById('modalTitle').textContent = logName;
      document.getElementById('modalLog').textContent = log;
      document.getElementById('logModal').classList.add('open');
    }
    
    function closeModal() {
      document.getElementById('logModal').classList.remove('open');
    }
    
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') closeModal();
    });
    
    function downloadReport() {
      const html = document.documentElement.outerHTML;
      const blob = new Blob(['<!DOCTYPE html>' + html], { type: 'text/html' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'agent-gym-report-' + new Date().toISOString().slice(0,10) + '.html';
      a.click();
      URL.revokeObjectURL(url);
    }

    // Smart refresh - fetch new page and swap body content
    ${isRunning ? `
    (function() {
      const REFRESH_INTERVAL = 2000;
      
      async function smartRefresh() {
        // Don't refresh if modal is open
        if (document.getElementById('logModal').classList.contains('open')) {
          setTimeout(smartRefresh, REFRESH_INTERVAL);
          return;
        }
        
        try {
          const response = await fetch(location.href + '?t=' + Date.now(), { cache: 'no-store' });
          const html = await response.text();
          
          // Parse the new HTML
          const parser = new DOMParser();
          const newDoc = parser.parseFromString(html, 'text/html');
          
          // Check if still running via data attribute
          const stillRunning = newDoc.body.dataset.running === 'true';
          
          // Replace entire body content but preserve scroll position
          const scrollY = window.scrollY;
          document.body.innerHTML = newDoc.body.innerHTML;
          document.body.dataset.running = newDoc.body.dataset.running;
          document.title = newDoc.title;
          window.scrollTo(0, scrollY);
          
          // Re-extract LOGS from the new script
          const newScript = newDoc.querySelector('script');
          if (newScript) {
            const scriptText = newScript.textContent;
            const startMarker = 'const LOGS = ';
            const startIdx = scriptText.indexOf(startMarker);
            if (startIdx !== -1) {
              const jsonStart = startIdx + startMarker.length;
              let depth = 0;
              let jsonEnd = jsonStart;
              for (let i = jsonStart; i < scriptText.length; i++) {
                if (scriptText[i] === '{') depth++;
                else if (scriptText[i] === '}') {
                  depth--;
                  if (depth === 0) {
                    jsonEnd = i + 1;
                    break;
                  }
                }
              }
              try {
                window.LOGS = JSON.parse(scriptText.slice(jsonStart, jsonEnd));
              } catch (e) {}
            }
          }
          
          if (stillRunning) {
            setTimeout(smartRefresh, REFRESH_INTERVAL);
          }
        } catch (e) {
          setTimeout(smartRefresh, REFRESH_INTERVAL * 2);
        }
      }
      
      setTimeout(smartRefresh, REFRESH_INTERVAL);
    })();
    ` : ''}
  </script>
</body>
</html>`;

  writeFileSync(outputPath, html);
  console.log(`\n📊 Report saved to: ${outputPath}`);
}

function printResults(results: TestResultWithLog[]): void {
  console.log("\n" + "=".repeat(60));
  console.log("RESULTS");
  console.log("=".repeat(60));

  for (const result of results) {
    const icon = result.run.status === "passed" ? "✓" : "✗";
    const { scenario, config } = result.run;
    console.log(
      `${icon} ${scenario.name} [${config.provider}/${config.model}] (${result.runnerName}) - ${result.run.status.toUpperCase()}`
    );

    for (const v of result.validations) {
      if (!v.passed) {
        console.log(`    ✗ ${v.message}`);
      }
    }
  }

  const passed = results.filter((r) => r.run.status === "passed").length;
  console.log(`\n${passed}/${results.length} tests passed`);
}

// =============================================================================
// Main
// =============================================================================

async function main() {
  // CLI --clear-cache: clear cache and exit
  if (process.argv.includes("--clear-cache")) {
    clearCache();
    return;
  }

  const rootDir = join(import.meta.dirname, "../..");
  const configPath = join(rootDir, "config.yaml");
  const scenariosDir = join(import.meta.dirname, "../scenarios");
  const workdir = join(import.meta.dirname, "../.workdir");
  const logsDir = join(rootDir, "logs");
  const reportPath = join(rootDir, "report.html");

  const config = loadConfig(configPath);
  let scenarios = loadAllScenarios(scenariosDir);

  // CLI --scenario= filter
  const scenarioFilter = process.argv.find((a) => a.startsWith("--scenario="))?.split("=")[1];
  if (scenarioFilter) {
    const filters = scenarioFilter.split(",");
    scenarios = scenarios.filter((s) => filters.some((f) => s.name.includes(f)));
  }

  // CLI --model= filter
  const modelFilter = process.argv.find((a) => a.startsWith("--model="))?.split("=")[1];
  if (modelFilter) {
    const filters = modelFilter.split(",");
    config.models = config.models.filter((m) => filters.some((f) => m.name.includes(f)));
  }

  // CLI --runner= filter
  const runnerFilter = process.argv.find((a) => a.startsWith("--runner="))?.split("=")[1];
  if (runnerFilter) {
    const filters = runnerFilter.split(",");
    config.runners = config.runners.filter((r) => filters.some((f) => r.name.includes(f)));
  }

  const pairs = buildTestPairs(config, scenarios);

  // Sort pairs by model name so same models run together (keeps model loaded in memory)
  pairs.sort((a, b) => a.model.name.localeCompare(b.model.name));

  // Show model grouping
  const modelOrder = [...new Set(pairs.map(p => p.model.name))];
  console.log(`\nExecution order (grouped by model for efficiency):`);
  for (const m of modelOrder) {
    const count = pairs.filter(p => p.model.name === m).length;
    console.log(`  ${m}: ${count} tests`);
  }

  // CLI --run-count=N (default 1)
  const runCountArg = process.argv.find((a) => a.startsWith("--run-count="))?.split("=")[1];
  const RUN_COUNT = runCountArg ? parseInt(runCountArg, 10) : 1;

  // CLI --no-cache: skip cache lookup (still stores results)
  const noCache = process.argv.includes("--no-cache");

  // Load cache and precompute hashes
  const cache = loadCache();
  const binaryHashes = new Map<string, string>();
  for (const runner of config.runners) {
    if (!binaryHashes.has(runner.bin)) {
      console.log(`Computing hash for ${runner.bin}...`);
      binaryHashes.set(runner.bin, getBinaryHash(runner.bin));
    }
  }
  const mcpHarnessHash = getMcpHarnessHash();

  console.log(`Models: ${config.models.map((m) => m.name).join(", ")}`);
  console.log(`Runners: ${config.runners.map((r) => r.name).join(", ")}`);
  console.log(`Running ${pairs.length} test pairs (${RUN_COUNT}x each, worst result kept)`);
  console.log(`Cache: ${noCache ? "disabled" : "enabled"} (${Object.keys(cache.entries).length} entries)`);

  const results: TestResultWithLog[] = [];
  
  // CLI --no-open to skip opening browser
  const noOpen = process.argv.includes("--no-open");

  let cacheHits = 0;
  let cacheMisses = 0;
  let browserOpened = false;

  for (const pair of pairs) {
    // Check cache first
    const { key: cacheKey, inputs: cacheInputs } = computeCacheKey(pair, binaryHashes, mcpHarnessHash);
    
    if (!noCache) {
      const cachedResult = getCachedResult(cache, cacheKey, pair, logsDir);
      if (cachedResult) {
        console.log(`\n${cachedResult.run.status === "passed" ? "✓" : "✗"} ${pair.scenario.name} [${pair.model.name}] (${pair.runner.name}) [CACHED]`);
        results.push(cachedResult);
        cacheHits++;
        continue;
      }
    }

    // First cache miss - generate report with cached results so far and open browser
    if (!browserOpened) {
      generateHtmlReport(results, reportPath, { isRunning: true, allPairs: pairs });
      if (!noOpen) {
        execSync(`open "${reportPath}"`);
      }
      browserOpened = true;
    }

    cacheMisses++;
    let worstResult: TestResultWithLog | null = null;

    for (let attempt = 1; attempt <= RUN_COUNT; attempt++) {
      console.log(`  Attempt ${attempt}/${RUN_COUNT} [${pair.runner.name}]`);
      const result = await runScenario(pair, workdir, logsDir, attempt);

      if (!worstResult) {
        worstResult = result;
      } else {
        const prevScore = scoreResult(worstResult);
        const currScore = scoreResult(result);
        if (currScore < prevScore) {
          worstResult = result;
        }
      }

      if (result.run.status === "failed") {
        break;
      }
    }

    // Store in cache
    storeCacheResult(cache, cacheKey, cacheInputs, worstResult!);

    results.push(worstResult!);
    generateHtmlReport(results, reportPath, { isRunning: true, allPairs: pairs });
  }

  generateHtmlReport(results, reportPath, { isRunning: false, allPairs: pairs });
  
  // If everything was cached, open browser now with final report
  if (!browserOpened && !noOpen) {
    execSync(`open "${reportPath}"`);
  }
  
  printResults(results);

  console.log(`\nCache summary: ${cacheHits} hits, ${cacheMisses} misses`);
}

main().catch(console.error);
