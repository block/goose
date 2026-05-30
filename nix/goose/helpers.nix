# Pure helper functions and shared option-type aliases for the goose module.
#
# Depends only on `lib`; no reference to `config`, so it is safe to import
# from option declarations and lowering alike.
{lib}: let
  inherit
    (builtins)
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
    concatStringsSep
    hasSuffix
    nameValuePair
    stringToCharacters
    toLower
    types
    ;
in rec {
  pathLikeType = types.coercedTo types.path toString types.str;
  environmentValueType = types.oneOf [
    types.bool
    types.float
    types.int
    types.path
    types.str
  ];

  immutableStateFileName = ".home-manager-state.json";
  environmentSecretFileName = ".home-manager-environment.sh";

  acpProviderRuntimeId = name:
    if hasSuffix "-acp" name
    then name
    else "${name}-acp";

  agentProviderRuntimeId = name:
    if hasSuffix "-agent" name
    then name
    else "${name}-agent";

  envValueToString = value:
    if isBool value
    then
      if value
      then "true"
      else "false"
    else if isInt value || isFloat value || isPath value
    then toString value
    else if isAttrs value || isList value
    then toJSON value
    else value;

  recipePathValueToString = value:
    if isList value
    then concatStringsSep ":" (map toString value)
    else envValueToString value;

  hoistSettingValue = name: value:
    if name == "GOOSE_RECIPE_PATH"
    then recipePathValueToString value
    else envValueToString value;

  normalizeEnvironment = attrs:
    lib.mapAttrs (_: envValueToString) attrs;

  uniquePackages = packages:
    lib.attrValues (lib.listToAttrs (map (pkg: nameValuePair (unsafeDiscardStringContext pkg.outPath) pkg) packages));

  storeFileName = name: builtins.replaceStrings ["/" " "] ["-" "-"] name;

  gooseNameToKey = name:
    toLower (concatStringsSep "" (map (char:
      if builtins.match "[A-Za-z0-9_-]" char != null
      then char
      else if builtins.match "[[:space:]]" char != null
      then ""
      else "_")
    (stringToCharacters name)));

  extensionTypeToRuntime = {
    builtin = "builtin";
    platform = "platform";
    stdio = "stdio";
    streamableHttp = "streamable_http";
  };

  hasNonDefaultPermissionSection = permissionCfg:
    permissionCfg.alwaysAllow
    != []
    || permissionCfg.askBefore != []
    || permissionCfg.neverAllow != [];

}
