# Submodule option types used by `programs.goose`.
{
  lib,
  pkgs,
  helpers,
}: let
  inherit (lib) literalExpression mkEnableOption mkOption types;
  inherit (helpers) environmentValueType pathLikeType;
  yamlFormat = pkgs.formats.yaml {};
  jsonFormat = pkgs.formats.json {};
in {
  promptTemplateSubmodule = types.submodule ({name, ...}: {
    options = {
      source = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = ''
          Path to the prompt template file to install as
          `${name}` in Goose's prompts directory.
        '';
      };

      text = mkOption {
        type = types.nullOr types.lines;
        default = null;
        description = ''
          Inline prompt template content for `${name}`.
        '';
      };
    };
  });

  permissionSectionSubmodule = types.submodule {
    options = {
      alwaysAllow = mkOption {
        type = types.listOf types.str;
        default = [];
      };

      askBefore = mkOption {
        type = types.listOf types.str;
        default = [];
      };

      neverAllow = mkOption {
        type = types.listOf types.str;
        default = [];
      };
    };
  };

  predefinedModelSubmodule = types.submodule {
    options = {
      id = mkOption {
        type = types.nullOr types.int;
        default = null;
      };

      name = mkOption {
        type = types.str;
      };

      provider = mkOption {
        type = types.str;
      };

      alias = mkOption {
        type = types.nullOr types.str;
        default = null;
      };

      subtext = mkOption {
        type = types.nullOr types.str;
        default = null;
      };

      contextLimit = mkOption {
        type = types.nullOr types.int;
        default = null;
      };

      requestParams = mkOption {
        type = types.nullOr jsonFormat.type;
        default = null;
      };
    };
  };

  providerSubmodule = types.submodule {
    options = {
      default = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Select this provider as Goose's default provider. At most one entry
          across `programs.goose.providers`,
          `programs.goose.acp.providers`, and
          `programs.goose.agent.providers` may set `default = true`.
        '';
      };

      settings = mkOption {
        inherit (yamlFormat) type;
        default = {};
        example = literalExpression ''
          {
            OPENAI_BASE_PATH = "/v1/chat/completions";
            OPENAI_TIMEOUT = 600;
          }
        '';
        description = ''
          Raw non-secret Goose config keys for this provider. Keys are written
          to Goose's `config.yaml` unchanged.
        '';
      };

      secretFiles = mkOption {
        type = types.attrsOf pathLikeType;
        default = {};
        example = literalExpression ''
          {
            OPENAI_API_KEY = config.age.secrets.openai-api-key.path;
          }
        '';
        description = ''
          Mapping of Goose secret key names to files containing the raw secret
          value. These are rendered into Goose's `secrets.yaml` by Home
          Manager.
        '';
      };
    };
  };

  extensionSubmodule = types.submodule ({name, ...}: {
    options = {
      enable = mkEnableOption "Goose extension ${name}";

      type = mkOption {
        type = types.enum [
          "stdio"
          "streamableHttp"
          "builtin"
          "platform"
        ];
        description = ''
          Goose extension transport/type to lower into `config.yaml`.
        '';
      };

      packages = mkOption {
        type = types.listOf types.package;
        default = [];
        defaultText = literalExpression "[]";
        description = ''
          Packages to install when this extension is enabled. Their `bin`
          directories are also prepended to Goose-managed search paths.
        '';
      };

      description = mkOption {
        type = types.str;
        default = "";
        description = ''
          Extension description written to Goose's `config.yaml`.
        '';
      };

      bundled = mkOption {
        type = types.nullOr types.bool;
        default = null;
        description = ''
          Optional `bundled` flag passed through to Goose unchanged.
        '';
      };

      availableTools = mkOption {
        type = types.listOf types.str;
        default = [];
        example = ["memory_read_graph"];
        description = ''
          Optional Goose tool filter lowered to `available_tools`.
        '';
      };

      command = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "npx";
        description = ''
          Command for `stdio` extensions. Required when `type = "stdio"`.
        '';
      };

      args = mkOption {
        type = types.listOf types.str;
        default = [];
        example = [
          "-y"
          "@modelcontextprotocol/server-memory"
        ];
        description = ''
          Arguments for `stdio` extensions.
        '';
      };

      environment = mkOption {
        type = types.attrsOf environmentValueType;
        default = {};
        example = literalExpression ''
          {
            MEMORY_PATH = "/var/lib/goose-memory";
          }
        '';
        description = ''
          Non-secret extension environment values lowered to Goose `envs`.
        '';
      };

      secretFiles = mkOption {
        type = types.attrsOf pathLikeType;
        default = {};
        example = literalExpression ''
          {
            API_KEY = config.age.secrets.example.path;
          }
        '';
        description = ''
          Secret file mappings for `stdio` and `streamableHttp` extensions.
          Keys are lowered to Goose `env_keys` and written into Goose's
          generated `secrets.yaml`.
        '';
      };

      timeout = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = ''
          Optional timeout in seconds for extension types that support it.
        '';
      };

      uri = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "https://mcp.example.com/stream";
        description = ''
          URI for `streamableHttp` extensions. Required when
          `type = "streamableHttp"`.
        '';
      };

      headers = mkOption {
        type = types.attrsOf types.str;
        default = {};
        example = literalExpression ''
          {
            Authorization = "Bearer $API_KEY";
          }
        '';
        description = ''
          Additional headers for `streamableHttp` extensions.
        '';
      };

      displayName = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          Optional display name for `builtin` and `platform` extensions.
        '';
      };
    };
  });

  acpProviderSubmodule = types.submodule ({name, ...}: {
    options = {
      enable = mkEnableOption "Goose ACP provider ${name}";

      default = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Select this ACP provider as Goose's default provider. At most one
          entry across `programs.goose.providers`,
          `programs.goose.acp.providers`, and
          `programs.goose.agent.providers` may set `default = true`.
        '';
      };

      packages = mkOption {
        type = types.listOf types.package;
        default = [];
        defaultText = literalExpression "[]";
        description = ''
          Packages to install when `programs.goose.acp.providers.${name}.enable = true`.

          Goose resolves ACP providers by running the matching command from
          `PATH`, so set this to the package(s) providing the
          `${name}-acp` adapter (and any agent CLI it wraps).
        '';
      };
    };
  });

  agentProviderSubmodule = types.submodule ({name, ...}: {
    options = {
      enable = mkEnableOption "Goose agent provider ${name}";

      default = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Select this agent provider as Goose's default provider. At most one
          entry across `programs.goose.providers`,
          `programs.goose.acp.providers`, and
          `programs.goose.agent.providers` may set `default = true`.
        '';
      };

      packages = mkOption {
        type = types.listOf types.package;
        default = [];
        defaultText = literalExpression "[]";
        description = ''
          Packages to install when `programs.goose.agent.providers.${name}.enable = true`.
        '';
      };
    };
  });
}
