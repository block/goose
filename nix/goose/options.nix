# `options.programs.goose` — the declarative surface of the goose module.
{
  lib,
  pkgs,
  self,
  helpers,
  gooseTypes,
  lowering,
}: let
  inherit (lib) literalExpression mkEnableOption mkOption types;
  inherit (helpers) environmentValueType pathLikeType;
  inherit
    (gooseTypes)
    acpProviderSubmodule
    agentProviderSubmodule
    extensionSubmodule
    permissionSectionSubmodule
    predefinedModelSubmodule
    promptTemplateSubmodule
    providerSubmodule
    ;
  inherit (lowering) gooseConfigDir gooseStateScript gooseStateSpec;
  inherit (pkgs.stdenv.hostPlatform) system;
  yamlFormat = pkgs.formats.yaml {};
  jsonFormat = pkgs.formats.json {};
in {
  programs.goose = {
    pathRoot = mkOption {
      type = types.nullOr pathLikeType;
      default = null;
      example = "/tmp/goose-test";
      description = ''
        Override Goose's root directory. Home Manager writes declarative Goose
        state to `$GOOSE_PATH_ROOT/config` when set and to
        `$XDG_CONFIG_HOME/goose` otherwise.
      '';
    };

    environment = mkOption {
      type = types.attrsOf environmentValueType;
      default = {};
      example = literalExpression ''
        {
          GOOSE_DEBUG = true;
          GOOSE_EDITOR_MODEL = "gpt-5";
        }
      '';
      description = ''
        Raw non-secret environment variables exported to Goose CLI and Desktop.
        Generated typed environment variables merge first, auto-hoisted
        `settings` values merge second, and this attrset merges last.
      '';
    };

    environmentSecretFiles = mkOption {
      type = types.attrsOf pathLikeType;
      default = {};
      example = literalExpression ''
        {
          GOOSE_EDITOR_API_KEY = config.age.secrets.openai-api-key.path;
        }
      '';
      description = ''
        Environment variables sourced from secret files at runtime for Goose
        CLI and Desktop.
      '';
    };

    generated = {
      configDir = mkOption {
        type = pathLikeType;
        readOnly = true;
        default = gooseConfigDir;
        description = ''
          Internal Goose Home Manager test/diagnostic output: the absolute
          Goose config directory managed by this module.
        '';
      };

      stateSpec = mkOption {
        type = pathLikeType;
        readOnly = true;
        default = gooseStateSpec;
        description = ''
          Internal Goose Home Manager test/diagnostic output: the generated
          Goose state spec JSON consumed by the activation hook.
        '';
      };

      stateScript = mkOption {
        type = pathLikeType;
        readOnly = true;
        default = gooseStateScript;
        description = ''
          Internal Goose Home Manager test/diagnostic output: the generated
          Goose state management script invoked during activation.
        '';
      };
    };

    acp = {
      providers = mkOption {
        type = types.attrsOf acpProviderSubmodule;
        default = {};
        example = literalExpression ''
          {
            claude.enable = true;
            copilot = {
              enable = true;
              packages = [ pkgs."github-copilot-cli" ];
            };
          }
        '';
        description = ''
          Declarative ACP provider presets keyed by ACP provider name without
          the `-acp` suffix. For example, `acp.providers.copilot` lowers to
          Goose provider id `copilot-acp`.
        '';
      };
    };

    agent = {
      providers = mkOption {
        type = types.attrsOf agentProviderSubmodule;
        default = {};
        example = literalExpression ''
          {
            cursor = {
              enable = true;
              default = true;
            };
          }
        '';
        description = ''
          Declarative non-ACP agent provider presets keyed by short agent name
          without the `-agent` suffix. For example, `agent.providers.cursor`
          lowers to Goose provider id `cursor-agent`.
        '';
      };
    };

    extensions = mkOption {
      type = types.attrsOf extensionSubmodule;
      default = {};
      example = literalExpression ''
        {
          developer = {
            enable = true;
            type = "builtin";
            bundled = true;
            timeout = 300;
          };

          memory = {
            enable = true;
            type = "stdio";
            command = "npx";
            args = [
              "-y"
              "@modelcontextprotocol/server-memory"
            ];
            packages = [ pkgs.nodejs ];
            secretFiles.API_KEY = config.age.secrets.memory-api-key.path;
          };
        }
      '';
      description = ''
        Declarative Goose extensions keyed by extension name. This typed
        surface covers `stdio`, `streamableHttp`, `builtin`, and `platform`
        extensions and lowers directly into Goose's `extensions` config.
        Unsupported Goose extension types remain available through
        `programs.goose.settings.extensions`.
      '';
    };

    cli.enable = mkEnableOption "Goose CLI";

    cli.package = mkOption {
      type = types.package;
      default = self.packages.${system}.default;
      defaultText = literalExpression "goose.packages.\${system}.default";
      description = ''
        Base Goose CLI package to wrap and install when Goose CLI is needed.

        Defaults to the `default` package built by this flake. Override it to
        supply a customised build (for example one with shell completions,
        man pages, or the `disable-update` feature enabled).
      '';
    };

    cli.theme = mkOption {
      type = types.nullOr (types.enum ["light" "dark" "ansi"]);
      default = null;
      description = ''
        Lowered to `GOOSE_CLI_THEME`.
      '';
    };

    cli.lightTheme = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = ''
        Lowered to `GOOSE_CLI_LIGHT_THEME`.
      '';
    };

    cli.darkTheme = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = ''
        Lowered to `GOOSE_CLI_DARK_THEME`.
      '';
    };

    cli.showCost = mkOption {
      type = types.nullOr types.bool;
      default = null;
      description = ''
        Lowered to `GOOSE_CLI_SHOW_COST`.
      '';
    };

    cli.minPriority = mkOption {
      type = types.nullOr (types.oneOf [types.int types.float]);
      default = null;
      description = ''
        Lowered to `GOOSE_CLI_MIN_PRIORITY`.
      '';
    };

    cli.newlineKey = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "n";
      description = ''
        Lowered to `GOOSE_CLI_NEWLINE_KEY`.
      '';
    };

    desktop.enable = mkEnableOption "Goose Desktop";

    desktop.package = mkOption {
      type = types.nullOr types.package;
      default = self.packages.${system}.goose-desktop or null;
      defaultText = literalExpression "goose.packages.\${system}.goose-desktop or null";
      description = ''
        Base Goose Desktop package to wrap and install when
        `programs.goose.desktop.enable = true`.

        Defaults to the `goose-desktop` package built by this flake when it is
        available for the host system, and to `null` otherwise. When enabling
        the desktop on a system without a prebuilt package, set this to your
        own Goose Desktop derivation.
      '';
    };

    providers = mkOption {
      type = types.attrsOf providerSubmodule;
      default = {};
      example = literalExpression ''
        {
          openai = {
            settings = {
              OPENAI_BASE_PATH = "/v1/chat/completions";
              OPENAI_TIMEOUT = 600;
            };
            secretFiles.OPENAI_API_KEY = config.age.secrets.openai-api-key.path;
          };

          ollama.settings.OLLAMA_HOST = "http://127.0.0.1:11434";
        }
      '';
      description = ''
        Declarative Goose provider configuration keyed by Goose's upstream
        provider ids. Each provider can contribute raw non-secret config keys
        and secret file mappings.
      '';
    };

    permissions = {
      user = mkOption {
        type = permissionSectionSubmodule;
        default = {};
      };

      smartApprove = mkOption {
        type = permissionSectionSubmodule;
        default = {};
      };
    };

    prompts.templates = mkOption {
      type = types.attrsOf promptTemplateSubmodule;
      default = {};
      example = literalExpression ''
        {
          "plan.md".text = "Include estimated time per step.";
        }
      '';
      description = ''
        Prompt template overrides written to Goose's `prompts/` directory.
      '';
    };

    recipes.paths = mkOption {
      type = types.listOf pathLikeType;
      default = [];
      example = [
        "/home/can/recipes"
        "/srv/team/goose-recipes"
      ];
      description = ''
        Additional recipe directories lowered to `GOOSE_RECIPE_PATH`.
      '';
    };

    recipes.githubRepo = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "block/goose-recipes";
      description = ''
        Lowered to `GOOSE_RECIPE_GITHUB_REPO`.
      '';
    };

    predefinedModels = mkOption {
      type = types.listOf predefinedModelSubmodule;
      default = [];
      description = ''
        Predefined model definitions lowered to `GOOSE_PREDEFINED_MODELS`.
      '';
    };

    allowlist = {
      url = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "https://example.com/allowlist.json";
      };

      warningMode = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Lowered to `GOOSE_ALLOWLIST_WARNING` for Goose Desktop.
        '';
      };
    };

    planner = {
      provider = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          Lowered to `GOOSE_PLANNER_PROVIDER`.
        '';
      };

      model = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          Lowered to `GOOSE_PLANNER_MODEL`.
        '';
      };
    };

    toolshim = {
      enable = mkOption {
        type = types.nullOr types.bool;
        default = null;
        description = ''
          Lowered to `GOOSE_TOOLSHIM`.
        '';
      };

      ollamaModel = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          Lowered to `GOOSE_TOOLSHIM_OLLAMA_MODEL`.
        '';
      };
    };

    searchPaths = mkOption {
      type = types.listOf pathLikeType;
      default = [];
      example = [
        "/usr/local/bin"
        "~/custom/tools"
      ];
      description = ''
        Additional search paths lowered to `GOOSE_SEARCH_PATHS` in
        Goose's `config.yaml`.
      '';
    };

    developer.shell = mkOption {
      type = types.nullOr pathLikeType;
      default = null;
      example = "/bin/zsh";
      description = ''
        Lowered to `GOOSE_SHELL`.
      '';
    };

    moim = {
      text = mkOption {
        type = types.nullOr types.lines;
        default = null;
      };

      file = mkOption {
        type = types.nullOr pathLikeType;
        default = null;
      };
    };

    promptEditor = {
      command = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "code --wait";
        description = ''
          Lowered to `GOOSE_PROMPT_EDITOR`.
        '';
      };

      always = mkOption {
        type = types.nullOr types.bool;
        default = null;
        description = ''
          Lowered to `GOOSE_PROMPT_EDITOR_ALWAYS`.
        '';
      };
    };

    session = {
      maxTurns = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = ''
          Lowered to `GOOSE_MAX_TURNS`.
        '';
      };

      autoCompactThreshold = mkOption {
        type = types.nullOr (types.oneOf [types.int types.float]);
        default = null;
        description = ''
          Lowered to `GOOSE_AUTO_COMPACT_THRESHOLD`.
        '';
      };

      maxActiveAgents = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = ''
          Lowered to `GOOSE_MAX_ACTIVE_AGENTS`.
        '';
      };

      disableSessionNaming = mkOption {
        type = types.nullOr types.bool;
        default = null;
        description = ''
          Lowered to `GOOSE_DISABLE_SESSION_NAMING`.
        '';
      };
    };

    subagents.maxTurns = mkOption {
      type = types.nullOr types.int;
      default = null;
      description = ''
        Lowered to `GOOSE_SUBAGENT_MAX_TURNS`.
      '';
    };

    telemetry.enable = mkOption {
      type = types.nullOr types.bool;
      default = null;
      description = ''
        Lowered to `GOOSE_TELEMETRY_ENABLED`.
      '';
    };

    security.promptInjection = {
      enable = mkOption {
        type = types.nullOr types.bool;
        default = null;
        description = ''
          Lowered to `SECURITY_PROMPT_ENABLED`.
        '';
      };

      threshold = mkOption {
        type = types.nullOr (types.oneOf [types.int types.float]);
        default = null;
        description = ''
          Lowered to `SECURITY_PROMPT_THRESHOLD`.
        '';
      };

      classifier = {
        enable = mkOption {
          type = types.nullOr types.bool;
          default = null;
          description = ''
            Lowered to `SECURITY_PROMPT_CLASSIFIER_ENABLED`.
          '';
        };

        endpoint = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = ''
            Lowered to `SECURITY_PROMPT_CLASSIFIER_ENDPOINT`.
          '';
        };
      };
    };

    terminalIntegration = {
      enable = mkEnableOption "Goose terminal integration";

      sessionName = mkOption {
        type = types.nullOr types.str;
        default = null;
      };

      commandNotFound = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Enable Goose's `--default` handler for unknown commands in Bash and
          Zsh terminal integration.
        '';
      };
    };

    defaultModel = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "gpt-5";
      description = ''
        Default Goose model string for the provider selected through
        `default = true`. Lowered to `GOOSE_MODEL` in Goose's `config.yaml`.
      '';
    };

    settings = mkOption {
      inherit (yamlFormat) type;
      default = {};
      example = literalExpression ''
        {
          GOOSE_PROVIDER = "anthropic";
          GOOSE_MODEL = "claude-sonnet-4-20250514";
          GOOSE_MODE = "smart_approve";

          extensions = {
            developer = {
              enabled = true;
              type = "builtin";
              name = "developer";
            };

            memory = {
              enabled = true;
              type = "stdio";
              name = "memory";
              cmd = "uvx";
              args = [ "mcp-server-memory" ];
              timeout = 300;
            };
          };
        }
      '';
      description = ''
        Raw escape hatch for Goose's `config.yaml`. Generated structured
        settings are merged first and these values are merged last.
      '';
    };

    customProviders = mkOption {
      type = types.attrsOf jsonFormat.type;
      default = {};
      example = literalExpression ''
        {
          llama-swap-local = {
            name = "llama-swap-local";
            engine = "openai";
            display_name = "Local llama-swap";
            base_url = "http://localhost:8013/v1";
            models = [
              { name = "qwen3-coder-next"; context_limit = 65536; }
            ];
            supports_streaming = true;
            requires_auth = false;
          };
        }
      '';
      description = ''
        Declarative provider definitions written to the resolved Goose
        config directory under `custom_providers/`.
      '';
    };
  };
}
