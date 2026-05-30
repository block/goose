# Home Manager module for goose, an open source, extensible AI agent.
#
# Usage (flake):
#
#   {
#     inputs.goose.url = "github:aaif-goose/goose";
#
#     outputs = { self, nixpkgs, home-manager, goose, ... }: {
#       homeConfigurations."you" = home-manager.lib.homeManagerConfiguration {
#         pkgs = import nixpkgs { system = "x86_64-linux"; };
#         modules = [
#           goose.homeManagerModules.default
#           {
#             programs.goose = {
#               cli.enable = true;
#               defaultModel = "claude-sonnet-4-20250514";
#               providers.anthropic.default = true;
#               extensions.developer = {
#                 enable = true;
#                 type = "builtin";
#               };
#             };
#           }
#         ];
#       };
#     };
#   }
#
# The `self` argument is this flake; `programs.goose.cli.package` and
# `programs.goose.desktop.package` default to the packages it exposes.
{self}: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.goose;
  inherit (pkgs.stdenv.hostPlatform) system;
  helpers = import ./helpers.nix {inherit lib;};
  gooseTypes = import ./types.nix {inherit lib pkgs helpers;};
  lowering = import ./lowering.nix {inherit lib pkgs config helpers;};

  inherit (builtins) hasAttr;
  inherit (lib) attrNames concatStringsSep escapeShellArg hasPrefix mkAfter mkIf mkMerge optional;
  inherit
    (lowering)
    activeAcpPackages
    activeAgentPackages
    activeExtensionPackages
    defaultProviderCandidates
    environmentSecretCollisions
    extensionKeyCollisionDetected
    extensionKeyCollisionNames
    extensionNames
    gooseRuntimeStateFile
    gooseStateScript
    gooseStateSpec
    gooseTermExe
    invalidAcpProviderPathNames
    invalidAgentProviderPathNames
    managedSecretFiles
    managesGooseState
    needsCliPackage
    providerExtensionSecretFileCollisions
    resolvedPathRoot
    resolvedStructuredDefaultProvider
    typedConflictAssertions
    wrappedCliPackage
    wrappedDesktopPackage
    ;
in {
  options = import ./options.nix {inherit lib pkgs self helpers gooseTypes lowering;};

  config = mkMerge [
    {
      assertions =
        [
          {
            assertion = builtins.length defaultProviderCandidates <= 1;
            message = "Exactly one provider across programs.goose.providers, programs.goose.acp.providers, and programs.goose.agent.providers may set default = true";
          }
          {
            assertion = invalidAcpProviderPathNames == [];
            message = "programs.goose.acp.providers must be keyed without the -acp suffix, for example programs.goose.acp.providers.copilot";
          }
          {
            assertion = invalidAgentProviderPathNames == [];
            message = "programs.goose.agent.providers must be keyed without the -agent suffix, for example programs.goose.agent.providers.cursor";
          }
          {
            assertion = resolvedStructuredDefaultProvider == null || cfg.defaultModel != null;
            message = "programs.goose.defaultModel must be set when selecting a default Goose provider";
          }
          {
            assertion = !(resolvedStructuredDefaultProvider != null && hasAttr "GOOSE_PROVIDER" cfg.settings);
            message = "Managed default Goose provider selection conflicts with programs.goose.settings.GOOSE_PROVIDER";
          }
          {
            assertion = !(resolvedStructuredDefaultProvider != null && hasAttr "GOOSE_PROVIDER" cfg.environment);
            message = "Managed default Goose provider selection conflicts with programs.goose.environment.GOOSE_PROVIDER";
          }
          {
            assertion = !(cfg.defaultModel != null && hasAttr "GOOSE_MODEL" cfg.settings);
            message = "programs.goose.defaultModel conflicts with programs.goose.settings.GOOSE_MODEL";
          }
          {
            assertion = !(cfg.defaultModel != null && hasAttr "GOOSE_MODEL" cfg.environment);
            message = "programs.goose.defaultModel conflicts with programs.goose.environment.GOOSE_MODEL";
          }
          {
            assertion = !(managedSecretFiles != {} && hasAttr "GOOSE_DISABLE_KEYRING" cfg.settings);
            message = "programs.goose.providers.<name>.secretFiles and programs.goose.extensions.<name>.secretFiles conflict with programs.goose.settings.GOOSE_DISABLE_KEYRING";
          }
          {
            assertion = !(managedSecretFiles != {} && hasAttr "GOOSE_DISABLE_KEYRING" cfg.environment);
            message = "programs.goose.providers.<name>.secretFiles and programs.goose.extensions.<name>.secretFiles conflict with programs.goose.environment.GOOSE_DISABLE_KEYRING";
          }
          {
            assertion = !(cfg.extensions != {} && hasAttr "extensions" cfg.settings);
            message = "programs.goose.extensions conflicts with programs.goose.settings.extensions";
          }
          {
            assertion = providerExtensionSecretFileCollisions == [];
            message = "programs.goose.providers.<name>.secretFiles and programs.goose.extensions.<name>.secretFiles must not reuse the same Goose secret keys: ${concatStringsSep ", " providerExtensionSecretFileCollisions}";
          }
          {
            assertion = !extensionKeyCollisionDetected;
            message = "programs.goose.extensions entries must normalize to unique Goose keys after name sanitization: ${concatStringsSep ", " extensionKeyCollisionNames}";
          }
          {
            assertion = resolvedPathRoot == null || hasPrefix "/" resolvedPathRoot;
            message = "programs.goose.pathRoot, programs.goose.settings.GOOSE_PATH_ROOT, and programs.goose.environment.GOOSE_PATH_ROOT must resolve to an absolute path";
          }
          {
            assertion = !cfg.terminalIntegration.commandNotFound || cfg.terminalIntegration.enable;
            message = "programs.goose.terminalIntegration.commandNotFound requires programs.goose.terminalIntegration.enable = true";
          }
          {
            assertion = !cfg.desktop.enable || cfg.desktop.package != null;
            message = "programs.goose.desktop.enable requires programs.goose.desktop.package to be set; this flake does not provide a Goose Desktop package for ${system}";
          }
          {
            assertion = environmentSecretCollisions == [];
            message = "programs.goose.environmentSecretFiles conflicts with non-secret Goose environment keys: ${concatStringsSep ", " environmentSecretCollisions}";
          }
        ]
        ++ typedConflictAssertions
        ++ map
        (name: let
          template = cfg.prompts.templates.${name};
        in {
          assertion = (template.source != null) != (template.text != null);
          message = "programs.goose.prompts.templates.${name} must set exactly one of `source` or `text`";
        })
        (attrNames cfg.prompts.templates)
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = extension.type != "stdio" || extension.command != null;
          message = "${extensionPath}.command is required when ${extensionPath}.type = \"stdio\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = extension.type == "stdio" || extension.command == null;
          message = "${extensionPath}.command is only valid when ${extensionPath}.type = \"stdio\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = extension.type == "stdio" || extension.args == [];
          message = "${extensionPath}.args is only valid when ${extensionPath}.type = \"stdio\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = builtins.elem extension.type ["stdio" "streamableHttp"] || extension.environment == {};
          message = "${extensionPath}.environment is only valid when ${extensionPath}.type = \"stdio\" or \"streamableHttp\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = builtins.elem extension.type ["stdio" "streamableHttp"] || extension.secretFiles == {};
          message = "${extensionPath}.secretFiles is only valid when ${extensionPath}.type = \"stdio\" or \"streamableHttp\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = extension.type != "streamableHttp" || extension.uri != null;
          message = "${extensionPath}.uri is required when ${extensionPath}.type = \"streamableHttp\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = extension.type == "streamableHttp" || extension.uri == null;
          message = "${extensionPath}.uri is only valid when ${extensionPath}.type = \"streamableHttp\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = extension.type == "streamableHttp" || extension.headers == {};
          message = "${extensionPath}.headers is only valid when ${extensionPath}.type = \"streamableHttp\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = builtins.elem extension.type ["builtin" "platform"] || extension.displayName == null;
          message = "${extensionPath}.displayName is only valid when ${extensionPath}.type = \"builtin\" or \"platform\"";
        })
        extensionNames
        ++ map
        (name: let
          extension = cfg.extensions.${name};
          extensionPath = "programs.goose.extensions.${name}";
        in {
          assertion = builtins.elem extension.type ["stdio" "streamableHttp" "builtin"] || extension.timeout == null;
          message = "${extensionPath}.timeout is only valid when ${extensionPath}.type = \"stdio\", \"streamableHttp\", or \"builtin\"";
        })
        extensionNames;

      home.packages =
        optional needsCliPackage wrappedCliPackage
        ++ activeAcpPackages
        ++ activeAgentPackages
        ++ activeExtensionPackages
        ++ optional cfg.desktop.enable wrappedDesktopPackage;
    }

    (mkIf managesGooseState {
      home.activation.gooseState = lib.hm.dag.entryAfter ["writeBoundary"] ''
        run ${escapeShellArg gooseStateScript} \
          ${escapeShellArg gooseStateSpec} \
          ${escapeShellArg gooseRuntimeStateFile}
      '';
    })

    (mkIf cfg.terminalIntegration.enable {
      programs.bash.initExtra = mkAfter ''
        eval "$(${gooseTermExe} term init bash${lib.optionalString (cfg.terminalIntegration.sessionName != null) " --name ${escapeShellArg cfg.terminalIntegration.sessionName}"}${lib.optionalString cfg.terminalIntegration.commandNotFound " --default"})"
      '';

      programs.fish.interactiveShellInit = mkAfter ''
        ${gooseTermExe} term init fish${lib.optionalString (cfg.terminalIntegration.sessionName != null) " --name ${escapeShellArg cfg.terminalIntegration.sessionName}"} | source
      '';

      programs.zsh.initContent = mkAfter ''
        eval "$(${gooseTermExe} term init zsh${lib.optionalString (cfg.terminalIntegration.sessionName != null) " --name ${escapeShellArg cfg.terminalIntegration.sessionName}"}${lib.optionalString cfg.terminalIntegration.commandNotFound " --default"})"
      '';
    })
  ];
}
