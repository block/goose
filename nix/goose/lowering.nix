# Lowering: turns `programs.goose` options into the config files, environment,
# wrapped packages and assertions the module installs at activation time.
{
  lib,
  pkgs,
  config,
  helpers,
}: let
  cfg = config.programs.goose;
  inherit
    (builtins)
    attrNames
    hasAttr
    isAttrs
    isBool
    isFloat
    isInt
    isList
    isPath
    toJSON
    unsafeDiscardStringContext
    ;
  inherit
    (lib)
    concatLists
    concatStringsSep
    escapeShellArg
    hasSuffix
    getExe
    getName
    hasPrefix
    literalExpression
    mapAttrs'
    mkAfter
    mkEnableOption
    mkIf
    mkMerge
    mkOption
    nameValuePair
    optional
    optionalAttrs
    optionals
    recursiveUpdate
    stringToCharacters
    toLower
    types
    unique
    ;
  inherit
    (helpers)
    acpProviderRuntimeId
    agentProviderRuntimeId
    environmentSecretFileName
    envValueToString
    extensionTypeToRuntime
    gooseNameToKey
    hasNonDefaultPermissionSection
    hoistSettingValue
    immutableStateFileName
    normalizeEnvironment
    recipePathValueToString
    storeFileName
    uniquePackages
    ;
  yamlFormat = pkgs.formats.yaml {};
  jsonFormat = pkgs.formats.json {};

  mkWrappedGoosePackage = {
    package,
    binary,
    extraPath ? [],
    environment ? {},
    environmentSecretFile ? null,
  }:
    pkgs.symlinkJoin {
      name = "${getName package}-goose-hm-wrapper";
      paths = [package];
      nativeBuildInputs = [pkgs.makeWrapper];
      meta =
        (package.meta or {})
        // {
          mainProgram = binary;
        };
      postBuild = let
        wrapperArgs =
          optional (extraPath != []) "--prefix PATH : ${escapeShellArg (lib.makeBinPath extraPath)}"
          ++ map (name: "--set ${escapeShellArg name} ${escapeShellArg environment.${name}}") (lib.sort builtins.lessThan (attrNames environment))
          ++ optional (environmentSecretFile != null) "--run ${escapeShellArg "if [ -f ${escapeShellArg environmentSecretFile} ]; then . ${escapeShellArg environmentSecretFile}; fi"}";
      in ''
        if [ ! -e "$out/bin/${binary}" ]; then
          echo "Expected $out/bin/${binary} to exist in ${getName package}" >&2
          exit 1
        fi

        wrapProgram "$out/bin/${binary}"${lib.optionalString (wrapperArgs != []) " \\\n          ${concatStringsSep " \\\n          " wrapperArgs}"}
      '';
    };

  cliExtraPath =
    [
      pkgs.bash
      pkgs.python3
    ]
    ++ optionals pkgs.stdenv.hostPlatform.isLinux [
      pkgs.xclip
      pkgs.xdotool
      pkgs.xwininfo
      pkgs.wmctrl
      pkgs.wtype
      pkgs.wl-clipboard
    ];

  desktopExtraPath =
    [
      pkgs.bash
      pkgs.coreutils
      pkgs.curl
      pkgs.gnused
      pkgs.gzip
      pkgs.python3
      pkgs.which
    ]
    ++ optionals pkgs.stdenv.hostPlatform.isLinux [
      pkgs.wtype
      pkgs.wl-clipboard
    ];

  providerNames = attrNames cfg.providers;
  extensionNames = attrNames cfg.extensions;
  extensionNormalizedNameMap = lib.mapAttrs' (name: _: nameValuePair name (gooseNameToKey name)) cfg.extensions;
  extensionKeyNames = attrNames extensionNormalizedNameMap;
  extensionKeyValues = lib.attrValues extensionNormalizedNameMap;
  extensionKeyCollisionDetected = builtins.length extensionKeyValues != builtins.length (unique extensionKeyValues);
  extensionKeyCollisionNames = lib.filter (name: let
    key = extensionNormalizedNameMap.${name};
  in
    builtins.length (lib.filter (other: extensionNormalizedNameMap.${other} == key) extensionKeyNames) > 1)
  extensionKeyNames;

  mergedProviderSettings =
    lib.foldl'
    recursiveUpdate
    {}
    (map (name: cfg.providers.${name}.settings) providerNames);

  generatedExtensions =
    mapAttrs'
    (name: extension:
      nameValuePair (gooseNameToKey name) (
        {
          enabled = extension.enable;
          type = extensionTypeToRuntime.${extension.type};
          inherit name;
          inherit (extension) description;
          available_tools = extension.availableTools;
        }
        // optionalAttrs (extension.bundled != null) {
          inherit (extension) bundled;
        }
        // (
          if extension.type == "stdio"
          then
            {
              cmd = extension.command;
              inherit (extension) args;
              envs = normalizeEnvironment extension.environment;
              env_keys = lib.sort builtins.lessThan (attrNames extension.secretFiles);
            }
            // optionalAttrs (extension.timeout != null) {
              inherit (extension) timeout;
            }
          else if extension.type == "streamableHttp"
          then
            {
              inherit (extension) uri;
              inherit (extension) headers;
              envs = normalizeEnvironment extension.environment;
              env_keys = lib.sort builtins.lessThan (attrNames extension.secretFiles);
            }
            // optionalAttrs (extension.timeout != null) {
              inherit (extension) timeout;
            }
          else if extension.type == "builtin"
          then
            optionalAttrs (extension.displayName != null) {
              display_name = extension.displayName;
            }
            // optionalAttrs (extension.timeout != null) {
              inherit (extension) timeout;
            }
          else
            optionalAttrs (extension.displayName != null) {
              display_name = extension.displayName;
            }
        )
      ))
    cfg.extensions;

  activeExtensionNames = lib.filter (name: cfg.extensions.${name}.enable) extensionNames;
  activeExtensionPackages = uniquePackages (concatLists (map (name: cfg.extensions.${name}.packages) activeExtensionNames));
  extensionSearchPaths = map (pkg: "${pkg}/bin") activeExtensionPackages;
  managedSearchPaths = unique (extensionSearchPaths ++ map toString cfg.searchPaths);

  providerManagedSecretFiles =
    lib.foldl'
    recursiveUpdate
    {}
    (map (name: cfg.providers.${name}.secretFiles) providerNames);

  extensionManagedSecretFiles =
    lib.foldl'
    recursiveUpdate
    {}
    (map (name: cfg.extensions.${name}.secretFiles) extensionNames);

  providerExtensionSecretFileCollisions =
    attrNames (lib.intersectAttrs providerManagedSecretFiles extensionManagedSecretFiles);

  managedSecretFiles = providerManagedSecretFiles // extensionManagedSecretFiles;

  rawSettingsPathRoot =
    if hasAttr "GOOSE_PATH_ROOT" cfg.settings
    then hoistSettingValue "GOOSE_PATH_ROOT" cfg.settings.GOOSE_PATH_ROOT
    else null;

  rawEnvironmentPathRoot =
    if hasAttr "GOOSE_PATH_ROOT" cfg.environment
    then envValueToString cfg.environment.GOOSE_PATH_ROOT
    else null;

  resolvedPathRoot =
    if cfg.pathRoot != null
    then toString cfg.pathRoot
    else if rawEnvironmentPathRoot != null
    then rawEnvironmentPathRoot
    else rawSettingsPathRoot;

  gooseConfigDir =
    if resolvedPathRoot != null
    then "${resolvedPathRoot}/config"
    else "${config.xdg.configHome}/goose";

  permissionConfig =
    optionalAttrs (hasNonDefaultPermissionSection cfg.permissions.user) {
      user = {
        always_allow = cfg.permissions.user.alwaysAllow;
        ask_before = cfg.permissions.user.askBefore;
        never_allow = cfg.permissions.user.neverAllow;
      };
    }
    // optionalAttrs (hasNonDefaultPermissionSection cfg.permissions.smartApprove) {
      smart_approve = {
        always_allow = cfg.permissions.smartApprove.alwaysAllow;
        ask_before = cfg.permissions.smartApprove.askBefore;
        never_allow = cfg.permissions.smartApprove.neverAllow;
      };
    };

  permissionFile = yamlFormat.generate "goose-permission.yaml" permissionConfig;

  resolvedAcpProviders =
    lib.mapAttrs'
    (name: provider:
      nameValuePair (acpProviderRuntimeId name) provider)
    cfg.acp.providers;

  invalidAcpProviderPathNames = lib.filter (name: hasSuffix "-acp" name) (attrNames cfg.acp.providers);
  declaredAcpProviderNames = attrNames resolvedAcpProviders;
  activeAcpProviders = lib.filterAttrs (_: provider: provider.enable) resolvedAcpProviders;
  activeAcpProviderNames = attrNames activeAcpProviders;
  activeAcpPackages = uniquePackages (concatLists (map (name: activeAcpProviders.${name}.packages) activeAcpProviderNames));

  resolvedAgentProviders =
    lib.mapAttrs'
    (name: provider:
      nameValuePair (agentProviderRuntimeId name) provider)
    cfg.agent.providers;

  invalidAgentProviderPathNames = lib.filter (name: hasSuffix "-agent" name) (attrNames cfg.agent.providers);
  declaredAgentProviderNames = attrNames resolvedAgentProviders;
  activeAgentProviders = lib.filterAttrs (_: provider: provider.enable) resolvedAgentProviders;
  activeAgentProviderNames = attrNames activeAgentProviders;
  activeAgentPackages = uniquePackages (concatLists (map (name: activeAgentProviders.${name}.packages) activeAgentProviderNames));

  defaultProviderCandidates =
    (map (name: name) (lib.filter (name: cfg.providers.${name}.default) providerNames))
    ++ (map (name: name) (lib.filter (name: resolvedAcpProviders.${name}.default) declaredAcpProviderNames))
    ++ (map (name: name) (lib.filter (name: resolvedAgentProviders.${name}.default) declaredAgentProviderNames));

  resolvedStructuredDefaultProvider =
    if defaultProviderCandidates == []
    then null
    else builtins.elemAt defaultProviderCandidates 0;

  generatedSettings =
    lib.foldl'
    recursiveUpdate
    {}
    [
      mergedProviderSettings
      (optionalAttrs (resolvedStructuredDefaultProvider != null) {
        GOOSE_PROVIDER = resolvedStructuredDefaultProvider;
      })
      (optionalAttrs (cfg.defaultModel != null) {
        GOOSE_MODEL = cfg.defaultModel;
      })
      (optionalAttrs (cfg.planner.provider != null) {
        GOOSE_PLANNER_PROVIDER = cfg.planner.provider;
      })
      (optionalAttrs (cfg.planner.model != null) {
        GOOSE_PLANNER_MODEL = cfg.planner.model;
      })
      (optionalAttrs (cfg.toolshim.enable != null) {
        GOOSE_TOOLSHIM = cfg.toolshim.enable;
      })
      (optionalAttrs (cfg.toolshim.ollamaModel != null) {
        GOOSE_TOOLSHIM_OLLAMA_MODEL = cfg.toolshim.ollamaModel;
      })
      (optionalAttrs (managedSearchPaths != []) {
        GOOSE_SEARCH_PATHS = managedSearchPaths;
      })
      (optionalAttrs (cfg.recipes.githubRepo != null) {
        GOOSE_RECIPE_GITHUB_REPO = cfg.recipes.githubRepo;
      })
      (optionalAttrs (cfg.promptEditor.command != null) {
        GOOSE_PROMPT_EDITOR = cfg.promptEditor.command;
      })
      (optionalAttrs (cfg.promptEditor.always != null) {
        GOOSE_PROMPT_EDITOR_ALWAYS = cfg.promptEditor.always;
      })
      (optionalAttrs (cfg.cli.theme != null) {
        GOOSE_CLI_THEME = cfg.cli.theme;
      })
      (optionalAttrs (cfg.cli.lightTheme != null) {
        GOOSE_CLI_LIGHT_THEME = cfg.cli.lightTheme;
      })
      (optionalAttrs (cfg.cli.darkTheme != null) {
        GOOSE_CLI_DARK_THEME = cfg.cli.darkTheme;
      })
      (optionalAttrs (cfg.cli.showCost != null) {
        GOOSE_CLI_SHOW_COST = cfg.cli.showCost;
      })
      (optionalAttrs (cfg.cli.minPriority != null) {
        GOOSE_CLI_MIN_PRIORITY = cfg.cli.minPriority;
      })
      (optionalAttrs (cfg.cli.newlineKey != null) {
        GOOSE_CLI_NEWLINE_KEY = cfg.cli.newlineKey;
      })
      (optionalAttrs (cfg.session.maxTurns != null) {
        GOOSE_MAX_TURNS = cfg.session.maxTurns;
      })
      (optionalAttrs (cfg.session.autoCompactThreshold != null) {
        GOOSE_AUTO_COMPACT_THRESHOLD = cfg.session.autoCompactThreshold;
      })
      (optionalAttrs (cfg.session.maxActiveAgents != null) {
        GOOSE_MAX_ACTIVE_AGENTS = cfg.session.maxActiveAgents;
      })
      (optionalAttrs (cfg.session.disableSessionNaming != null) {
        GOOSE_DISABLE_SESSION_NAMING = cfg.session.disableSessionNaming;
      })
      (optionalAttrs (cfg.subagents.maxTurns != null) {
        GOOSE_SUBAGENT_MAX_TURNS = cfg.subagents.maxTurns;
      })
      (optionalAttrs (cfg.telemetry.enable != null) {
        GOOSE_TELEMETRY_ENABLED = cfg.telemetry.enable;
      })
      (optionalAttrs (cfg.security.promptInjection.enable != null) {
        SECURITY_PROMPT_ENABLED = cfg.security.promptInjection.enable;
      })
      (optionalAttrs (cfg.security.promptInjection.threshold != null) {
        SECURITY_PROMPT_THRESHOLD = cfg.security.promptInjection.threshold;
      })
      (optionalAttrs (cfg.security.promptInjection.classifier.enable != null) {
        SECURITY_PROMPT_CLASSIFIER_ENABLED = cfg.security.promptInjection.classifier.enable;
      })
      (optionalAttrs (cfg.security.promptInjection.classifier.endpoint != null) {
        SECURITY_PROMPT_CLASSIFIER_ENDPOINT = cfg.security.promptInjection.classifier.endpoint;
      })
      (optionalAttrs (managedSecretFiles != {}) {
        GOOSE_DISABLE_KEYRING = true;
      })
      (optionalAttrs (cfg.extensions != {}) {
        extensions = generatedExtensions;
      })
    ];

  finalSettings = recursiveUpdate generatedSettings cfg.settings;

  predefinedModelsValue =
    map (
      model:
        {
          inherit (model) name;
          inherit (model) provider;
        }
        // optionalAttrs (model.id != null) {inherit (model) id;}
        // optionalAttrs (model.alias != null) {inherit (model) alias;}
        // optionalAttrs (model.subtext != null) {inherit (model) subtext;}
        // optionalAttrs (model.contextLimit != null) {context_limit = model.contextLimit;}
        // optionalAttrs (model.requestParams != null) {request_params = model.requestParams;}
    )
    cfg.predefinedModels;

  generatedEnvironment = normalizeEnvironment (
    (optionalAttrs (resolvedPathRoot != null) {
      GOOSE_PATH_ROOT = resolvedPathRoot;
    })
    // optionalAttrs (cfg.allowlist.url != null) {
      GOOSE_ALLOWLIST = cfg.allowlist.url;
    }
    // optionalAttrs cfg.allowlist.warningMode {
      GOOSE_ALLOWLIST_WARNING = true;
    }
    // optionalAttrs (cfg.recipes.paths != []) {
      GOOSE_RECIPE_PATH = concatStringsSep ":" (map toString cfg.recipes.paths);
    }
    // optionalAttrs (cfg.predefinedModels != []) {
      GOOSE_PREDEFINED_MODELS = toJSON predefinedModelsValue;
    }
    // optionalAttrs (cfg.developer.shell != null) {
      GOOSE_SHELL = cfg.developer.shell;
    }
    // optionalAttrs (cfg.moim.text != null) {
      GOOSE_MOIM_MESSAGE_TEXT = cfg.moim.text;
    }
    // optionalAttrs (cfg.moim.file != null) {
      GOOSE_MOIM_MESSAGE_FILE = cfg.moim.file;
    }
  );

  hoistedSettingEnvironment =
    lib.mapAttrs'
    (name: value: nameValuePair name (hoistSettingValue name value))
    (lib.filterAttrs (name: _:
      builtins.elem name [
        "GOOSE_ALLOWLIST"
        "GOOSE_ALLOWLIST_WARNING"
        "GOOSE_MOIM_MESSAGE_FILE"
        "GOOSE_MOIM_MESSAGE_TEXT"
        "GOOSE_PATH_ROOT"
        "GOOSE_PREDEFINED_MODELS"
        "GOOSE_RECIPE_PATH"
        "GOOSE_SHELL"
      ])
    cfg.settings);

  finalEnvironment = generatedEnvironment // hoistedSettingEnvironment // normalizeEnvironment cfg.environment;

  promptFiles =
    mapAttrs' (
      name: template:
        nameValuePair "prompts/${name}" (
          if template.text != null
          then pkgs.writeText "goose-prompt-${storeFileName name}" template.text
          else template.source
        )
    )
    cfg.prompts.templates;

  customProviderFiles =
    mapAttrs' (
      name: value:
        nameValuePair "custom_providers/${name}.json" (
          jsonFormat.generate "goose-provider-${storeFileName name}.json" value
        )
    )
    cfg.customProviders;

  editableFiles =
    (optionalAttrs (finalSettings != {}) {
      "config.yaml" = yamlFormat.generate "goose-config.yaml" finalSettings;
    })
    // (optionalAttrs (permissionConfig != {}) {
      "permission.yaml" = permissionFile;
    })
    // (lib.mapAttrs (_: value: value) customProviderFiles);

  immutableFiles = lib.mapAttrs (_: value: value) promptFiles;

  gooseStateSpec = pkgs.writeText "goose-home-manager-spec.json" (toJSON {
    configDir = gooseConfigDir;
    inherit environmentSecretFileName;
    immutableFiles = lib.mapAttrs (_: toString) immutableFiles;
    editableFiles = lib.mapAttrs (_: toString) editableFiles;
    managedSecretFiles = lib.mapAttrs (_: toString) managedSecretFiles;
    managedEnvironmentSecretFiles = lib.mapAttrs (_: toString) cfg.environmentSecretFiles;
    stateFileName = immutableStateFileName;
  });

  gooseRuntimeStateFile = "${config.xdg.stateHome}/home-manager/goose/${immutableStateFileName}";
  gooseEnvironmentSecretFile = "${gooseConfigDir}/${environmentSecretFileName}";

  managesGooseState =
    cfg.cli.enable
    || cfg.desktop.enable
    || cfg.terminalIntegration.enable
    || finalSettings != {}
    || permissionConfig != {}
    || cfg.customProviders != {}
    || cfg.prompts.templates != {}
    || finalEnvironment != {}
    || cfg.environmentSecretFiles != {}
    || managedSecretFiles != {}
    || activeAcpProviderNames != []
    || activeAgentProviderNames != [];

  needsCliPackage = cfg.cli.enable || cfg.terminalIntegration.enable;

  wrappedCliPackage = mkWrappedGoosePackage {
    inherit (cfg.cli) package;
    binary = "goose";
    extraPath = cliExtraPath;
    environment = finalEnvironment;
    environmentSecretFile =
      if cfg.environmentSecretFiles != {}
      then gooseEnvironmentSecretFile
      else null;
  };

  wrappedDesktopPackage = mkWrappedGoosePackage {
    inherit (cfg.desktop) package;
    binary = "goose-desktop";
    extraPath = desktopExtraPath;
    environment = finalEnvironment;
    environmentSecretFile =
      if cfg.environmentSecretFiles != {}
      then gooseEnvironmentSecretFile
      else null;
  };

  gooseTermExe = escapeShellArg (getExe wrappedCliPackage);


  gooseStateScript = pkgs.writeShellScript "goose-home-manager-state" ''
    set -euo pipefail
    ${pkgs.python3}/bin/python ${./manage-state.py} "$1" "$2"
  '';

  mkTypedSettingConflictAssertions = {
    enabled,
    optionPath,
    keys,
  }:
    concatLists [
      (map
        (key: {
          assertion = !enabled || !hasAttr key cfg.settings;
          message = "${optionPath} conflicts with programs.goose.settings.${key}";
        })
        keys)
      (map
        (key: {
          assertion = !enabled || !hasAttr key cfg.environment;
          message = "${optionPath} conflicts with programs.goose.environment.${key}";
        })
        keys)
    ];

  typedConflictAssertions = concatLists [
    (mkTypedSettingConflictAssertions {
      enabled = cfg.pathRoot != null;
      optionPath = "programs.goose.pathRoot";
      keys = ["GOOSE_PATH_ROOT"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.allowlist.url != null;
      optionPath = "programs.goose.allowlist.url";
      keys = ["GOOSE_ALLOWLIST"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.allowlist.warningMode;
      optionPath = "programs.goose.allowlist.warningMode";
      keys = ["GOOSE_ALLOWLIST_WARNING"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.recipes.paths != [];
      optionPath = "programs.goose.recipes.paths";
      keys = ["GOOSE_RECIPE_PATH"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.recipes.githubRepo != null;
      optionPath = "programs.goose.recipes.githubRepo";
      keys = ["GOOSE_RECIPE_GITHUB_REPO"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.predefinedModels != [];
      optionPath = "programs.goose.predefinedModels";
      keys = ["GOOSE_PREDEFINED_MODELS"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.planner.provider != null;
      optionPath = "programs.goose.planner.provider";
      keys = ["GOOSE_PLANNER_PROVIDER"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.planner.model != null;
      optionPath = "programs.goose.planner.model";
      keys = ["GOOSE_PLANNER_MODEL"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.toolshim.enable != null;
      optionPath = "programs.goose.toolshim.enable";
      keys = ["GOOSE_TOOLSHIM"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.toolshim.ollamaModel != null;
      optionPath = "programs.goose.toolshim.ollamaModel";
      keys = ["GOOSE_TOOLSHIM_OLLAMA_MODEL"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = managedSearchPaths != [];
      optionPath = "managed Goose search paths from programs.goose.searchPaths and enabled programs.goose.extensions.<name>.packages";
      keys = ["GOOSE_SEARCH_PATHS"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.developer.shell != null;
      optionPath = "programs.goose.developer.shell";
      keys = ["GOOSE_SHELL"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.moim.text != null;
      optionPath = "programs.goose.moim.text";
      keys = ["GOOSE_MOIM_MESSAGE_TEXT"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.moim.file != null;
      optionPath = "programs.goose.moim.file";
      keys = ["GOOSE_MOIM_MESSAGE_FILE"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.promptEditor.command != null;
      optionPath = "programs.goose.promptEditor.command";
      keys = ["GOOSE_PROMPT_EDITOR"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.promptEditor.always != null;
      optionPath = "programs.goose.promptEditor.always";
      keys = ["GOOSE_PROMPT_EDITOR_ALWAYS"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.cli.theme != null;
      optionPath = "programs.goose.cli.theme";
      keys = ["GOOSE_CLI_THEME"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.cli.lightTheme != null;
      optionPath = "programs.goose.cli.lightTheme";
      keys = ["GOOSE_CLI_LIGHT_THEME"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.cli.darkTheme != null;
      optionPath = "programs.goose.cli.darkTheme";
      keys = ["GOOSE_CLI_DARK_THEME"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.cli.showCost != null;
      optionPath = "programs.goose.cli.showCost";
      keys = ["GOOSE_CLI_SHOW_COST"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.cli.minPriority != null;
      optionPath = "programs.goose.cli.minPriority";
      keys = ["GOOSE_CLI_MIN_PRIORITY"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.cli.newlineKey != null;
      optionPath = "programs.goose.cli.newlineKey";
      keys = ["GOOSE_CLI_NEWLINE_KEY"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.session.maxTurns != null;
      optionPath = "programs.goose.session.maxTurns";
      keys = ["GOOSE_MAX_TURNS"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.session.autoCompactThreshold != null;
      optionPath = "programs.goose.session.autoCompactThreshold";
      keys = ["GOOSE_AUTO_COMPACT_THRESHOLD"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.session.maxActiveAgents != null;
      optionPath = "programs.goose.session.maxActiveAgents";
      keys = ["GOOSE_MAX_ACTIVE_AGENTS"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.session.disableSessionNaming != null;
      optionPath = "programs.goose.session.disableSessionNaming";
      keys = ["GOOSE_DISABLE_SESSION_NAMING"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.subagents.maxTurns != null;
      optionPath = "programs.goose.subagents.maxTurns";
      keys = ["GOOSE_SUBAGENT_MAX_TURNS"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.telemetry.enable != null;
      optionPath = "programs.goose.telemetry.enable";
      keys = ["GOOSE_TELEMETRY_ENABLED"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.security.promptInjection.enable != null;
      optionPath = "programs.goose.security.promptInjection.enable";
      keys = ["SECURITY_PROMPT_ENABLED"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.security.promptInjection.threshold != null;
      optionPath = "programs.goose.security.promptInjection.threshold";
      keys = ["SECURITY_PROMPT_THRESHOLD"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.security.promptInjection.classifier.enable != null;
      optionPath = "programs.goose.security.promptInjection.classifier.enable";
      keys = ["SECURITY_PROMPT_CLASSIFIER_ENABLED"];
    })
    (mkTypedSettingConflictAssertions {
      enabled = cfg.security.promptInjection.classifier.endpoint != null;
      optionPath = "programs.goose.security.promptInjection.classifier.endpoint";
      keys = ["SECURITY_PROMPT_CLASSIFIER_ENDPOINT"];
    })
  ];

  environmentSecretCollisions =
    attrNames (lib.intersectAttrs finalEnvironment cfg.environmentSecretFiles);
in {
  inherit
    activeAcpPackages
    activeAgentPackages
    activeExtensionPackages
    defaultProviderCandidates
    environmentSecretCollisions
    extensionKeyCollisionDetected
    extensionKeyCollisionNames
    extensionNames
    gooseConfigDir
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
}
