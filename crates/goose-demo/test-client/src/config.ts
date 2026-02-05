/**
 * Agent configuration for test-client
 * 
 * Supports named presets and custom commands for testing different ACP agents.
 */

export interface AgentConfig {
  name: string;
  command: string;
  args: string[];
  description?: string;
}

/**
 * Built-in agent presets
 * 
 * Add new presets here for commonly tested agents.
 */
export const AGENT_PRESETS: Record<string, AgentConfig> = {
  // goose2 - the new Rust implementation (default)
  goose2: {
    name: "goose2",
    command: "cargo",
    args: ["run", "--manifest-path", "../Cargo.toml"],
    description: "goose2 Rust agent (development build)",
  },

  // goose2 release build
  "goose2-release": {
    name: "goose2-release",
    command: "cargo",
    args: ["run", "--release", "--manifest-path", "../Cargo.toml"],
    description: "goose2 Rust agent (release build)",
  },

  // Original goose CLI (assumes it's installed globally or in PATH)
  goose: {
    name: "goose",
    command: "goose",
    args: ["acp", "--with-builtin", "develop"],
    description: "Original goose CLI agent",
  },

  // Goose from a specific path (useful for testing local builds)
  "goose-local": {
    name: "goose-local",
    command: process.env.GOOSE_PATH || "goose",
    args: ["acp"],
    description: "Original goose from GOOSE_PATH or default",
  },
};

/**
 * Default agent to use when none specified
 * Can be overridden with AGENT environment variable
 */
export const DEFAULT_AGENT = process.env.AGENT || "goose2";

/**
 * Parse command line arguments and return agent configuration
 */
export function parseArgs(argv: string[]): { config: AgentConfig; showHelp: boolean } {
  const args = argv.slice(2);

  // Check for help flag
  if (args.includes("--help") || args.includes("-h")) {
    return { config: AGENT_PRESETS[DEFAULT_AGENT], showHelp: true };
  }

  // Check for --agent flag
  const agentFlagIndex = args.findIndex((a) => a === "--agent" || a === "-a");
  if (agentFlagIndex !== -1) {
    const agentName = args[agentFlagIndex + 1];
    if (!agentName) {
      console.error("Error: --agent requires a value");
      process.exit(1);
    }

    const preset = AGENT_PRESETS[agentName];
    if (!preset) {
      console.error(`Error: Unknown agent preset '${agentName}'`);
      console.error(`Available presets: ${Object.keys(AGENT_PRESETS).join(", ")}`);
      process.exit(1);
    }

    return { config: preset, showHelp: false };
  }

  // Check for --list flag
  if (args.includes("--list") || args.includes("-l")) {
    console.log("Available agent presets:\n");
    for (const [name, config] of Object.entries(AGENT_PRESETS)) {
      const isDefault = name === DEFAULT_AGENT ? " (default)" : "";
      console.log(`  ${name}${isDefault}`);
      console.log(`    Command: ${config.command} ${config.args.join(" ")}`);
      if (config.description) {
        console.log(`    ${config.description}`);
      }
      console.log();
    }
    process.exit(0);
  }

  // If raw command provided (no flags), use it directly
  if (args.length > 0 && !args[0].startsWith("-")) {
    const [command, ...commandArgs] = args;
    return {
      config: {
        name: "custom",
        command,
        args: commandArgs,
        description: "Custom command from CLI",
      },
      showHelp: false,
    };
  }

  // No args - use default agent
  if (args.length === 0) {
    const preset = AGENT_PRESETS[DEFAULT_AGENT];
    if (!preset) {
      console.error(`Error: Default agent '${DEFAULT_AGENT}' not found in presets`);
      process.exit(1);
    }
    return { config: preset, showHelp: false };
  }

  // Unknown flags
  console.error(`Error: Unknown argument '${args[0]}'`);
  return { config: AGENT_PRESETS[DEFAULT_AGENT], showHelp: true };
}

/**
 * Print help message
 */
export function printHelp(): void {
  console.log(`
goose2 test client - ACP agent testing TUI

USAGE:
  npm start                           Use default agent (${DEFAULT_AGENT})
  npm start -- --agent <preset>       Use a named preset
  npm start -- <command> [args...]    Use a custom command

OPTIONS:
  -a, --agent <name>    Use a named agent preset
  -l, --list            List available agent presets
  -h, --help            Show this help message

ENVIRONMENT VARIABLES:
  AGENT                 Default agent preset (current: ${DEFAULT_AGENT})
  GOOSE_PATH            Path to goose binary for 'goose-local' preset

EXAMPLES:
  # Test goose2 (default)
  npm start

  # Test original goose
  npm start -- --agent goose

  # Test with custom command
  npm start -- /path/to/my-agent --some-flag

  # Set default agent via environment
  AGENT=goose npm start

PRESETS:
${Object.entries(AGENT_PRESETS)
  .map(([name, config]) => {
    const isDefault = name === DEFAULT_AGENT ? " (default)" : "";
    return `  ${name}${isDefault}\n    ${config.command} ${config.args.join(" ")}`;
  })
  .join("\n")}
`);
}
