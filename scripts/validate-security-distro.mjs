import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const requiredSkillIds = [
  "vuln-triage",
  "alert-triage",
  "ioc-analysis",
  "asset-risk-summary",
  "report-writing",
  "wooyun-legacy",
];
const requiredRecipeIds = [
  "security-vuln-triage",
  "alert-investigation",
  "ioc-analysis",
  "web-investigation",
  "report-writing",
  "wooyun-legacy",
];
const requiredDesktopTaskMappings = [
  {
    taskId: "vuln-triage",
    skillId: "vuln-triage",
    recipeId: "security-vuln-triage",
    recommendedExtensions: ["aiseesec-mcp"],
  },
  {
    taskId: "alert-investigation",
    skillId: "alert-triage",
    recipeId: "alert-investigation",
    recommendedExtensions: ["threat-intel-mcp", "local-security-gateway-mcp"],
  },
  {
    taskId: "ioc-analysis",
    skillId: "ioc-analysis",
    recipeId: "ioc-analysis",
    recommendedExtensions: ["threat-intel-mcp"],
  },
  {
    taskId: "web-investigation",
    skillId: "ioc-analysis",
    recipeId: "web-investigation",
    recommendedExtensions: ["browser-assist-mcp", "threat-intel-mcp"],
  },
  {
    taskId: "report-writing",
    skillId: "report-writing",
    recipeId: "report-writing",
    recommendedExtensions: [],
  },
  {
    taskId: "wooyun-legacy",
    skillId: "wooyun-legacy",
    recipeId: "wooyun-legacy",
    recommendedExtensions: ["browser-assist-mcp"],
  },
];
const requiredSecurityExtensionIds = [
  "aiseesec-mcp",
  "local-security-gateway-mcp",
  "threat-intel-mcp",
  "browser-assist-mcp",
];
const realPreviewSecurityExtensionIds = [
  "threat-intel-mcp",
  "browser-assist-mcp",
];
const blockedSecurityExtensionIds = [
  "aiseesec-mcp",
  "local-security-gateway-mcp",
];
const allowedRuntimeSkillExtraPrefixes = new Map([
  ["wooyun-legacy", ["external/upstream"]],
]);

const requiredFiles = [
  "docs/v1a/README.md",
  "docs/v1a/11-bootstrap-audit.md",
  "docs/v1a/examples/init-config.yaml.example",
  "docs/v1a/examples/desktop-env.example",
  "docs/v1a/examples/bundled-extensions.security.json.example",
  "docs/v1a/examples/security-vuln-triage.recipe.yaml.example",
  "distro/security-cn/README.md",
  "distro/security-cn/branding/README.md",
  "distro/security-cn/branding/product-metadata.json",
  "distro/security-cn/config/init-config.yaml.example",
  "distro/security-cn/config/desktop-env.example",
  "distro/security-cn/config/provider-defaults.yaml",
  "distro/security-cn/config/model-catalog.json",
  "distro/security-cn/config/feature-flags.json",
  "distro/security-cn/locales/zh-CN.json",
  "distro/security-cn/locales/en-US.json",
  "distro/security-cn/prompts/system-zh.md",
  "distro/security-cn/prompts/system-en.md",
  "distro/security-cn/prompts/security-role-defaults.md",
  "distro/security-cn/skills/README.md",
  "distro/security-cn/skills/wooyun-legacy/SKILL.md",
  "distro/security-cn/skills/wooyun-legacy/external/README.md",
  "distro/security-cn/recipes/README.md",
  "distro/security-cn/extensions/README.md",
  "distro/security-cn/extensions/bundled-extensions.security.json.example",
  "distro/security-cn/docs/operator-guide.md",
  "distro/security-cn/docs/capability-catalog.md",
  "distro/security-cn/docs/signed-release-handoff-panel.md",
  "distro/security-cn/docs/signed-release-runbook.md",
  ".github/workflows/security-goose-v1a-checks.yml",
  "scripts/check-security-github-release-readiness.mjs",
  "scripts/check-security-v1a.sh",
  "scripts/render-security-macos-release-evidence.mjs",
  "scripts/smoke-security-extensions.mjs",
  "scripts/install-wooyun-legacy-skill.mjs",
  "ui/desktop/src/branding/desktopBrandAssets.test.ts",
  "ui/desktop/src/components/icons/Goose.test.tsx",
  "ui/desktop/src/images/brand-mark.svg",
  "ui/desktop/src/images/generate-brand-assets.mjs",
  "ui/desktop/src/components/recipes/RecipesView.test.tsx",
];

const jsonFiles = [
  "distro/security-cn/branding/product-metadata.json",
  "distro/security-cn/config/model-catalog.json",
  "distro/security-cn/config/feature-flags.json",
  "distro/security-cn/locales/zh-CN.json",
  "distro/security-cn/locales/en-US.json",
  "distro/security-cn/extensions/bundled-extensions.security.json.example",
  "docs/v1a/examples/bundled-extensions.security.json.example",
];

function readFile(relPath) {
  return fs.readFileSync(path.join(repoRoot, relPath), "utf8");
}

function normalizeLineEndings(value) {
  return value.replace(/\r\n/g, "\n");
}

function listRelativeFiles(rootDir, currentDir = rootDir) {
  const relativeFiles = [];

  for (const entry of fs.readdirSync(currentDir, { withFileTypes: true })) {
    const entryPath = path.join(currentDir, entry.name);
    if (entry.isDirectory()) {
      relativeFiles.push(...listRelativeFiles(rootDir, entryPath));
      continue;
    }

    if (entry.isFile()) {
      relativeFiles.push(path.relative(rootDir, entryPath));
    }
  }

  return relativeFiles.sort();
}

function isAllowedRuntimeSkillExtra(skillId, relativePath) {
  const prefixes = allowedRuntimeSkillExtraPrefixes.get(skillId) ?? [];
  return prefixes.some(
    (prefix) => relativePath === prefix || relativePath.startsWith(`${prefix}${path.sep}`),
  );
}

function parseEnvFile(contents) {
  const parsed = {};
  for (const rawLine of contents.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }

    const separatorIndex = line.indexOf("=");
    if (separatorIndex <= 0) {
      continue;
    }

    const key = line.slice(0, separatorIndex).trim();
    let value = line.slice(separatorIndex + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    parsed[key] = value;
  }
  return parsed;
}

function parseSimpleConfigFile(contents, separator) {
  const parsed = {};
  for (const rawLine of contents.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }

    const separatorIndex = line.indexOf(separator);
    if (separatorIndex <= 0) {
      continue;
    }

    const key = line.slice(0, separatorIndex).trim();
    const value = line.slice(separatorIndex + separator.length).trim();
    parsed[key] = value;
  }
  return parsed;
}

function parseSkillFrontmatter(raw) {
  const match = raw.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!match) {
    throw new Error("Skill file is missing frontmatter");
  }

  const metadata = {};
  for (const line of match[1].split(/\r?\n/)) {
    const separatorIndex = line.indexOf(":");
    if (separatorIndex <= 0) {
      continue;
    }
    const key = line.slice(0, separatorIndex).trim();
    const value = line
      .slice(separatorIndex + 1)
      .trim()
      .replace(/^['"]|['"]$/g, "");
    metadata[key] = value;
  }
  return metadata;
}

for (const relPath of requiredFiles) {
  const fullPath = path.join(repoRoot, relPath);
  if (!fs.existsSync(fullPath)) {
    throw new Error(`Missing required file: ${relPath}`);
  }
}

for (const relPath of jsonFiles) {
  JSON.parse(readFile(relPath));
}

const brandMark = readFile("ui/desktop/src/images/brand-mark.svg");
if (!brandMark.includes("IBM Carbon AiEnabledEdt")) {
  throw new Error("ui/desktop/src/images/brand-mark.svg is not pinned to the Carbon AiEnabledEdt source");
}

const generatedGlyph = readFile("ui/desktop/src/images/glyph.svg");
if (!generatedGlyph.includes("Generated from brand-mark.svg via generate-brand-assets.mjs")) {
  throw new Error("ui/desktop/src/images/glyph.svg is no longer marked as generated from brand-mark.svg");
}

const generatedIcon = readFile("ui/desktop/src/images/icon.svg");
if (!generatedIcon.includes("Generated from brand-mark.svg via generate-brand-assets.mjs")) {
  throw new Error("ui/desktop/src/images/icon.svg is no longer marked as generated from brand-mark.svg");
}

const imagePrepareScript = readFile("ui/desktop/src/images/prepare.sh");
if (!imagePrepareScript.includes("generate-brand-assets.mjs")) {
  throw new Error("ui/desktop/src/images/prepare.sh no longer regenerates branding svg assets first");
}

const readme = readFile("docs/v1a/README.md");
if (readme.includes("/Users/nano/git/CSO")) {
  throw new Error("docs/v1a/README.md still points to the old CSO path");
}

for (const relPath of [
  "docs/v1a/examples/init-config.yaml.example",
  "distro/security-cn/config/init-config.yaml.example",
]) {
  const content = readFile(relPath);
  if (content.includes("GOOSE_DEFAULT_LOCALE")) {
    throw new Error(`${relPath} still uses GOOSE_DEFAULT_LOCALE`);
  }
  if (content.includes("GOOSE_PREDEFINED_MODELS: auto,")) {
    throw new Error(`${relPath} still uses comma-separated predefined models`);
  }
}

const gitignore = readFile(".gitignore");
if (!gitignore.includes(".agents/skills/wooyun-legacy/external/upstream/*")) {
  throw new Error(
    ".gitignore must ignore installed wooyun-legacy upstream runtime assets",
  );
}

for (const relPath of [
  "docs/v1a/examples/bundled-extensions.security.json.example",
  "distro/security-cn/extensions/bundled-extensions.security.json.example",
]) {
  const entries = JSON.parse(readFile(relPath));
  for (const entry of entries) {
    if (entry.id !== entry.name) {
      throw new Error(`${relPath} contains mismatched id/name for ${entry.id}`);
    }
  }
}

const modelCatalog = JSON.parse(
  readFile("distro/security-cn/config/model-catalog.json"),
);
if (!Array.isArray(modelCatalog) || modelCatalog.length === 0) {
  throw new Error("model-catalog.json must be a non-empty array");
}

for (const model of modelCatalog) {
  if (!model.name || !model.provider) {
    throw new Error("Every model catalog entry must include name and provider");
  }
}
const expectedTokenPlanModelIds = [
  "auto",
  "deepseek-v4-flash",
  "deepseek-v4-flash-202605",
  "deepseek-v4-pro",
  "deepseek-v4-pro-202606",
  "glm-5",
  "glm-5-turbo",
  "glm-5.1",
  "kimi-k2.5",
  "kimi-k2.6",
  "minimax-m2.5",
  "minimax-m2.7",
  "minimax-m3",
];
if (
  JSON.stringify(modelCatalog.map((model) => model.name)) !==
  JSON.stringify(expectedTokenPlanModelIds)
) {
  throw new Error(
    "model-catalog.json must stay aligned with the curated Token Plan model id list",
  );
}

const productMetadata = JSON.parse(
  readFile("distro/security-cn/branding/product-metadata.json"),
);
if (!productMetadata.productName || !productMetadata.defaultLocale) {
  throw new Error(
    "product-metadata.json must include productName and defaultLocale",
  );
}

const desktopPackage = JSON.parse(readFile("ui/desktop/package.json"));
if (desktopPackage.productName !== productMetadata.productName) {
  throw new Error(
    "ui/desktop/package.json productName must match branding/product-metadata.json",
  );
}

const viteMainConfig = readFile("ui/desktop/vite.main.config.mts");
if (!viteMainConfig.includes("productMetadata.productName")) {
  throw new Error(
    "ui/desktop/vite.main.config.mts must derive GOOSE_BUNDLE_NAME from product metadata",
  );
}

const githubUpdaterSource = readFile("ui/desktop/src/utils/githubUpdater.ts");
if (!githubUpdaterSource.includes("loadSecurityDistroDefaults")) {
  throw new Error(
    "ui/desktop/src/utils/githubUpdater.ts must derive bundle names from distro defaults",
  );
}

for (const relPath of [
  ".github/workflows/bundle-desktop.yml",
  ".github/workflows/bundle-desktop-intel.yml",
  ".github/workflows/bundle-desktop-windows.yml",
]) {
  const workflow = readFile(relPath);
  if (!workflow.includes("bundle_metadata")) {
    throw new Error(`${relPath} must resolve bundle metadata before packaging`);
  }
}

const desktopEnv = parseEnvFile(
  readFile("distro/security-cn/config/desktop-env.example"),
);
if (desktopEnv.GOOSE_LOCALE !== productMetadata.defaultLocale) {
  throw new Error(
    "desktop-env.example locale must match branding/product-metadata.json",
  );
}
if (!desktopEnv.GOOSE_DEFAULT_PROVIDER || !desktopEnv.GOOSE_DEFAULT_MODEL) {
  throw new Error("desktop-env.example must define default provider and model");
}
if (desktopEnv.GOOSE_TELEMETRY_ENABLED !== "true") {
  throw new Error(
    "desktop-env.example must default GOOSE_TELEMETRY_ENABLED to true for Security Goose preview telemetry",
  );
}
if (desktopEnv.GOOSE_POSTHOG_API_HOST !== "https://us.i.posthog.com") {
  throw new Error(
    "desktop-env.example must pin GOOSE_POSTHOG_API_HOST to https://us.i.posthog.com",
  );
}
if (
  desktopEnv.GOOSE_POSTHOG_PROJECT_API_KEY !==
  "phc_yS3ZTSB2WBmKf6aiBHstbfV4Nc2cxc7KxVavBxNjBBSn"
) {
  throw new Error(
    "desktop-env.example must pin GOOSE_POSTHOG_PROJECT_API_KEY to the Security Goose PostHog project token",
  );
}

const predefinedModels = JSON.parse(desktopEnv.GOOSE_PREDEFINED_MODELS);
if (!Array.isArray(predefinedModels) || predefinedModels.length === 0) {
  throw new Error(
    "desktop-env.example must define a non-empty GOOSE_PREDEFINED_MODELS array",
  );
}
if (predefinedModels.length !== modelCatalog.length) {
  throw new Error(
    "GOOSE_PREDEFINED_MODELS must stay in sync with model-catalog.json",
  );
}
if (JSON.stringify(predefinedModels) !== JSON.stringify(modelCatalog)) {
  throw new Error(
    "desktop-env.example GOOSE_PREDEFINED_MODELS must exactly match model-catalog.json",
  );
}

const providerDefaults = parseSimpleConfigFile(
  readFile("distro/security-cn/config/provider-defaults.yaml"),
  ":",
);
const initConfig = parseSimpleConfigFile(
  readFile("distro/security-cn/config/init-config.yaml.example"),
  ":",
);

if (providerDefaults.provider !== desktopEnv.GOOSE_DEFAULT_PROVIDER) {
  throw new Error(
    "provider-defaults.yaml provider must match desktop-env.example default provider",
  );
}
if (providerDefaults.default_model !== desktopEnv.GOOSE_DEFAULT_MODEL) {
  throw new Error(
    "provider-defaults.yaml default_model must match desktop-env.example default model",
  );
}
if (providerDefaults.desktop_locale !== desktopEnv.GOOSE_LOCALE) {
  throw new Error(
    "provider-defaults.yaml desktop_locale must match desktop-env.example locale",
  );
}
if (initConfig.GOOSE_PROVIDER !== providerDefaults.provider) {
  throw new Error(
    "init-config.yaml.example GOOSE_PROVIDER must match provider-defaults.yaml provider",
  );
}
if (initConfig.OPENAI_BASE_URL !== providerDefaults.base_url) {
  throw new Error(
    "init-config.yaml.example OPENAI_BASE_URL must match provider-defaults.yaml base_url",
  );
}
if (initConfig.GOOSE_MODEL !== providerDefaults.default_model) {
  throw new Error(
    "init-config.yaml.example GOOSE_MODEL must match provider-defaults.yaml default_model",
  );
}
if (initConfig.GOOSE_TELEMETRY_ENABLED !== "true") {
  throw new Error(
    "init-config.yaml.example must default GOOSE_TELEMETRY_ENABLED to true for Security Goose preview telemetry",
  );
}
if (initConfig.GOOSE_POSTHOG_API_HOST !== "https://us.i.posthog.com") {
  throw new Error(
    "init-config.yaml.example must pin GOOSE_POSTHOG_API_HOST to https://us.i.posthog.com",
  );
}
if (
  initConfig.GOOSE_POSTHOG_PROJECT_API_KEY !==
  "phc_yS3ZTSB2WBmKf6aiBHstbfV4Nc2cxc7KxVavBxNjBBSn"
) {
  throw new Error(
    "init-config.yaml.example must pin GOOSE_POSTHOG_PROJECT_API_KEY to the Security Goose PostHog project token",
  );
}

const docsInitConfig = parseSimpleConfigFile(
  readFile("docs/v1a/examples/init-config.yaml.example"),
  ":",
);
if (JSON.stringify(docsInitConfig) !== JSON.stringify(initConfig)) {
  throw new Error(
    "docs/v1a/examples/init-config.yaml.example must stay in sync with distro/security-cn/config/init-config.yaml.example",
  );
}

for (const skillId of requiredSkillIds) {
  const sourceDir = path.join(repoRoot, "distro/security-cn/skills", skillId);
  const runtimeDir = path.join(repoRoot, ".agents/skills", skillId);
  const sourcePath = path.join("distro/security-cn/skills", skillId, "SKILL.md");
  const runtimePath = path.join(".agents/skills", skillId, "SKILL.md");
  const sourceContents = normalizeLineEndings(readFile(sourcePath));
  const runtimeContents = normalizeLineEndings(readFile(runtimePath));
  const frontmatter = parseSkillFrontmatter(sourceContents);
  const sourceFiles = listRelativeFiles(sourceDir);
  const runtimeFiles = listRelativeFiles(runtimeDir);

  if (frontmatter.name !== skillId) {
    throw new Error(`${sourcePath} frontmatter name must match ${skillId}`);
  }
  if (!frontmatter.description) {
    throw new Error(`${sourcePath} must include a non-empty description`);
  }

  for (const heading of [
    "## 使用场景",
    "## 输入要求",
    "## 执行步骤",
    "## 输出模板",
    "## 风险与边界",
    "## 验证步骤",
  ]) {
    if (!sourceContents.includes(heading)) {
      throw new Error(`${sourcePath} is missing required section ${heading}`);
    }
  }

  if (sourceContents !== runtimeContents) {
    throw new Error(`${runtimePath} must stay in sync with ${sourcePath}`);
  }

  for (const relativePath of sourceFiles) {
    const sourceFilePath = path.join(sourceDir, relativePath);
    const runtimeFilePath = path.join(runtimeDir, relativePath);
    if (!fs.existsSync(runtimeFilePath)) {
      throw new Error(
        `${path.relative(repoRoot, runtimeFilePath)} is missing mirrored file ${relativePath}`,
      );
    }

    const sourceFileContents = normalizeLineEndings(
      fs.readFileSync(sourceFilePath, "utf8"),
    );
    const runtimeFileContents = normalizeLineEndings(
      fs.readFileSync(runtimeFilePath, "utf8"),
    );

    if (sourceFileContents !== runtimeFileContents) {
      throw new Error(
        `${path.relative(repoRoot, runtimeFilePath)} must stay in sync with ${path.relative(
          repoRoot,
          sourceFilePath,
        )}`,
      );
    }
  }

  const unexpectedRuntimeFiles = runtimeFiles.filter(
    (relativePath) =>
      !sourceFiles.includes(relativePath) && !isAllowedRuntimeSkillExtra(skillId, relativePath),
  );

  if (unexpectedRuntimeFiles.length > 0) {
    throw new Error(
      `${path.relative(repoRoot, runtimeDir)} contains unexpected mirrored files: ${unexpectedRuntimeFiles.join(
        ", ",
      )}`,
    );
  }
}

const wooyunExternalReadme = readFile(
  "distro/security-cn/skills/wooyun-legacy/external/README.md",
);
if (!wooyunExternalReadme.includes("CC BY-NC-SA 4.0")) {
  throw new Error(
    "wooyun-legacy external README must document the upstream CC BY-NC-SA 4.0 license",
  );
}
if (!wooyunExternalReadme.includes("scripts/install-wooyun-legacy-skill.mjs")) {
  throw new Error(
    "wooyun-legacy external README must point to the local install script",
  );
}

for (const recipeId of requiredRecipeIds) {
  const sourcePath = `distro/security-cn/recipes/${recipeId}.yaml.example`;
  const runtimePath = `.goose/recipes/${recipeId}.yaml`;
  const sourceContents = normalizeLineEndings(readFile(sourcePath));
  const runtimeContents = normalizeLineEndings(readFile(runtimePath));

  for (const requiredField of ["title:", "description:", "instructions:", "extensions:"]) {
    if (!sourceContents.includes(requiredField)) {
      throw new Error(`${sourcePath} must include ${requiredField}`);
    }
  }

  if (/^skills:/m.test(sourceContents)) {
    throw new Error(`${sourcePath} must not use unsupported top-level skills field`);
  }

  if (sourceContents !== runtimeContents) {
    throw new Error(`${runtimePath} must stay in sync with ${sourcePath}`);
  }
}

const securityCatalog = JSON.parse(
  readFile("distro/security-cn/extensions/bundled-extensions.security.json.example"),
);
if (securityCatalog.length !== requiredSecurityExtensionIds.length) {
  throw new Error("Security extension catalog must include the first four preview entries");
}

const desktopBundledCatalog = JSON.parse(
  readFile("ui/desktop/src/components/settings/extensions/bundled-extensions.json"),
);

for (const extensionId of requiredSecurityExtensionIds) {
  const sourceEntry = securityCatalog.find((entry) => entry.id === extensionId);
  if (!sourceEntry) {
    throw new Error(`Missing ${extensionId} in distro security extension catalog`);
  }
  if (sourceEntry.enabled !== false) {
    throw new Error(`${extensionId} must default to disabled in the source catalog`);
  }

  const serverPath = sourceEntry.args?.[0];
  if (!serverPath) {
    throw new Error(`${extensionId} must point to a local server.mjs stub`);
  }
  if (!fs.existsSync(path.join(repoRoot, serverPath))) {
    throw new Error(`${extensionId} points to missing file ${serverPath}`);
  }

  const desktopEntry = desktopBundledCatalog.find((entry) => entry.id === extensionId);
  if (!desktopEntry) {
    throw new Error(
      `ui/desktop/src/components/settings/extensions/bundled-extensions.json is missing ${extensionId}`,
    );
  }

  for (const key of ["id", "name", "display_name", "description", "type", "cmd", "timeout"]) {
    if (desktopEntry[key] !== sourceEntry[key]) {
      throw new Error(`${extensionId} desktop bundled entry differs on ${key}`);
    }
  }

  if (desktopEntry.enabled !== false) {
    throw new Error(`${extensionId} desktop bundled entry must default to disabled`);
  }
}

for (const extensionId of realPreviewSecurityExtensionIds) {
  const serverPath = securityCatalog.find((entry) => entry.id === extensionId)?.args?.[0];
  if (!serverPath) {
    throw new Error(`${extensionId} must point to a local server.mjs file`);
  }

  const serverSource = readFile(serverPath);
  if (serverSource.includes("Goal 5 preview stub") || serverSource.includes("process.exit(1)")) {
    throw new Error(`${extensionId} must no longer be a stub server`);
  }
}

for (const extensionId of blockedSecurityExtensionIds) {
  const serverPath = securityCatalog.find((entry) => entry.id === extensionId)?.args?.[0];
  if (!serverPath) {
    throw new Error(`${extensionId} must point to a local server.mjs stub`);
  }

  const serverSource = readFile(serverPath);
  if (!serverSource.includes("Goal 5 preview stub") || !serverSource.includes("process.exit(1)")) {
    throw new Error(`${extensionId} must remain an explicit disabled stub in Goal 8`);
  }
}

const threatIntelSourceEntry = securityCatalog.find((entry) => entry.id === "threat-intel-mcp");
if ((threatIntelSourceEntry?.env_keys ?? []).length !== 0) {
  throw new Error("threat-intel-mcp must stay zero-config for Goal 8 local preview");
}

const taskCatalogSource = readFile("ui/desktop/src/security/taskCatalog.ts");
for (const taskMapping of requiredDesktopTaskMappings) {
  if (!taskCatalogSource.includes(`id: '${taskMapping.taskId}'`)) {
    throw new Error(`taskCatalog.ts is missing desktop task ${taskMapping.taskId}`);
  }
  if (!taskCatalogSource.includes(`skillId: '${taskMapping.skillId}'`)) {
    throw new Error(
      `taskCatalog.ts is missing skill mapping ${taskMapping.skillId} for ${taskMapping.taskId}`,
    );
  }
  if (
    taskMapping.recipeId &&
    !taskCatalogSource.includes(`recipeId: '${taskMapping.recipeId}'`)
  ) {
    throw new Error(
      `taskCatalog.ts is missing recipe mapping ${taskMapping.recipeId} for ${taskMapping.taskId}`,
    );
  }

  for (const extensionId of taskMapping.recommendedExtensions) {
    if (!taskCatalogSource.includes(`'${extensionId}'`)) {
      throw new Error(
        `taskCatalog.ts is missing recommended extension ${extensionId} for ${taskMapping.taskId}`,
      );
    }
  }
}

const launcherViewSource = readFile("ui/desktop/src/components/LauncherView.tsx");
if (!launcherViewSource.includes("SECURITY_TASK_IDS.map")) {
  throw new Error("LauncherView.tsx must render the curated security task catalog");
}
if (!launcherViewSource.includes("window.electron.createChatWindow")) {
  throw new Error("LauncherView.tsx must launch security tasks through createChatWindow");
}
if (!launcherViewSource.includes("SecurityExtensionOverview")) {
  throw new Error("LauncherView.tsx must render the Goal 9 extension status overview");
}
if (!launcherViewSource.includes("SecurityTaskExtensionHints")) {
  throw new Error("LauncherView.tsx must render task-level extension recommendation hints");
}

const recipesViewSource = readFile("ui/desktop/src/components/recipes/RecipesView.tsx");
if (!recipesViewSource.includes("listSavedRecipes")) {
  throw new Error("RecipesView.tsx must keep using Goose-native saved recipe discovery");
}
if (!recipesViewSource.includes("handleStartRecipeChat")) {
  throw new Error("RecipesView.tsx must keep the existing Goose recipe launch path");
}
if (!recipesViewSource.includes("savedRecipesTitle")) {
  throw new Error("RecipesView.tsx must present the native saved recipes section");
}
if (recipesViewSource.includes("SECURITY_TASK_IDS.map")) {
  throw new Error("RecipesView.tsx must not reintroduce a parallel security task starter panel");
}
if (recipesViewSource.includes("SecurityExtensionOverview")) {
  throw new Error("RecipesView.tsx must not duplicate extension overview UI above the native recipe list");
}
if (recipesViewSource.includes("setView('extensions')")) {
  throw new Error("RecipesView.tsx must not add a parallel extensions shortcut in place of Goose-native recipes");
}

const expectedBuiltInSecurityApps = [
  "ioc-toolbox",
  "encode-hash-lab",
  "secret-credential-scanner",
  "jwt-inspector",
];
const legacyDefaultApps = ["clock", "chat"];

const defaultAppsSource = readFile("crates/goose/src/goose_apps/default_apps.rs");
for (const appName of expectedBuiltInSecurityApps) {
  if (!defaultAppsSource.includes(`"${appName}"`)) {
    throw new Error(`default_apps.rs must include curated security app ${appName}`);
  }
}
for (const legacyApp of legacyDefaultApps) {
  if (!defaultAppsSource.includes(`"${legacyApp}"`)) {
    throw new Error(`default_apps.rs must continue to track legacy app ${legacyApp} for cleanup`);
  }
}

const appsViewSource = readFile("ui/desktop/src/components/apps/AppsView.tsx");
for (const appName of expectedBuiltInSecurityApps) {
  if (!appsViewSource.includes(`'${appName}'`)) {
    throw new Error(`AppsView.tsx must recognize curated security app ${appName}`);
  }
}
for (const requiredSnippet of [
  "apps-built-in-security-section",
  "apps-imported-custom-section",
  "Built-in security tools",
  "Imported / custom apps",
]) {
  if (!appsViewSource.includes(requiredSnippet)) {
    throw new Error(`AppsView.tsx must keep the curated security app sectioning (${requiredSnippet})`);
  }
}

const operatorGuide = readFile("distro/security-cn/docs/operator-guide.md");
for (const requiredSnippet of [
  "scripts/check-security-v1a.sh",
  "scripts/smoke-security-extensions.mjs",
  "scripts/check-security-apps-runtime.mjs",
  "scripts/run-security-visual-apps-smoke.sh",
  "pnpm --dir ui/desktop run bundle:default",
  "pnpm --dir ui/desktop run bundle:intel",
  "gateway",
  "LiteLLM",
  "AGS",
  "在线 marketplace",
  "企业后台",
]) {
  if (!operatorGuide.includes(requiredSnippet)) {
    throw new Error(
      `operator-guide.md must document the Goal 7 check, packaging path, and non-goals (${requiredSnippet})`,
    );
  }
}

const testingDoc = readFile("docs/v1a/07-testing-ci-cd.md");
for (const requiredSnippet of [
  "security-goose-v1a-checks.yml",
  "scripts/check-security-v1a.sh",
  "scripts/smoke-security-extensions.mjs",
  "macOS-only",
]) {
  if (!testingDoc.includes(requiredSnippet)) {
    throw new Error(`07-testing-ci-cd.md must mention ${requiredSnippet}`);
  }
}

const workflowSource = readFile(".github/workflows/security-goose-v1a-checks.yml");
if (!workflowSource.includes("runs-on: macos-latest")) {
  throw new Error("security-goose-v1a-checks.yml must run on macOS");
}
const workflowUsesSharedScript = workflowSource.includes("./scripts/check-security-v1a.sh");
const workflowUsesEquivalentSteps = [
  "Install UI workspace dependencies",
  "Build UI SDK",
  "Sync Security runtime assets",
  "Smoke Security extension wiring",
  "Check Apple signing boundary",
  "Run desktop V1a tests",
  "Run desktop typecheck",
  "Run desktop lint",
  "Validate Security distro",
  "Check tracked diffs",
  "Run packaged bundle smoke",
].every((requiredSnippet) => workflowSource.includes(requiredSnippet));
if (!workflowUsesSharedScript && !workflowUsesEquivalentSteps) {
  throw new Error(
    "security-goose-v1a-checks.yml must execute the shared security V1a check script or an equivalent explicit Security Goose V1a step chain",
  );
}

const checkScript = readFile("scripts/check-security-v1a.sh");
if (!checkScript.includes("node scripts/smoke-security-extensions.mjs")) {
  throw new Error("scripts/check-security-v1a.sh must run extension smoke validation");
}

const extensionReadme = readFile("distro/security-cn/extensions/README.md");
for (const requiredSnippet of [
  "browser-assist-mcp",
  "threat-intel-mcp",
  "disabled stub",
  "blocker",
  "绝对路径",
]) {
  if (!extensionReadme.includes(requiredSnippet)) {
    throw new Error(`extensions/README.md must document ${requiredSnippet}`);
  }
}

console.log("security-cn distro skeleton validation passed");
