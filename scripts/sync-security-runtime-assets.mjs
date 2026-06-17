import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const sourceSkillRoot = path.join(repoRoot, "distro/security-cn/skills");
const runtimeSkillRoot = path.join(repoRoot, ".agents/skills");
const sourceRecipeRoot = path.join(repoRoot, "distro/security-cn/recipes");
const runtimeRecipeRoot = path.join(repoRoot, ".goose/recipes");
const allowedRuntimeSkillExtraPrefixes = new Map([
  ["wooyun-legacy", ["external/upstream"]],
]);

function copyFile(sourcePath, targetPath) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, fs.readFileSync(sourcePath, "utf8"));
}

function copyDirectory(sourceDir, targetDir) {
  fs.mkdirSync(targetDir, { recursive: true });

  for (const entry of fs.readdirSync(sourceDir, { withFileTypes: true })) {
    const sourcePath = path.join(sourceDir, entry.name);
    const targetPath = path.join(targetDir, entry.name);

    if (entry.isDirectory()) {
      copyDirectory(sourcePath, targetPath);
      continue;
    }

    if (entry.isFile()) {
      copyFile(sourcePath, targetPath);
    }
  }
}

function isAllowedRuntimeSkillExtra(skillDir, relativePath) {
  const prefixes = allowedRuntimeSkillExtraPrefixes.get(skillDir) ?? [];
  return prefixes.some(
    (prefix) => relativePath === prefix || relativePath.startsWith(`${prefix}${path.sep}`),
  );
}

function pruneMirroredSkillDirectory(sourceDir, targetDir, skillDir, currentDir = targetDir) {
  let prunedEntries = 0;

  if (!fs.existsSync(currentDir)) {
    return prunedEntries;
  }

  for (const entry of fs.readdirSync(currentDir, { withFileTypes: true })) {
    const runtimePath = path.join(currentDir, entry.name);
    const relativePath = path.relative(targetDir, runtimePath);
    const sourcePath = path.join(sourceDir, relativePath);

    if (!fs.existsSync(sourcePath)) {
      if (isAllowedRuntimeSkillExtra(skillDir, relativePath)) {
        continue;
      }

      fs.rmSync(runtimePath, { recursive: true, force: true });
      prunedEntries += 1;
      continue;
    }

    if (entry.isDirectory()) {
      prunedEntries += pruneMirroredSkillDirectory(sourceDir, targetDir, skillDir, runtimePath);
    }
  }

  return prunedEntries;
}

function syncSkills() {
  const skillDirs = fs
    .readdirSync(sourceSkillRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .filter((name) => fs.existsSync(path.join(sourceSkillRoot, name, "SKILL.md")));

  for (const skillDir of skillDirs) {
    copyDirectory(
      path.join(sourceSkillRoot, skillDir),
      path.join(runtimeSkillRoot, skillDir),
    );
    pruneMirroredSkillDirectory(
      path.join(sourceSkillRoot, skillDir),
      path.join(runtimeSkillRoot, skillDir),
      skillDir,
    );
  }

  return skillDirs;
}

function syncRecipes() {
  const recipeFiles = fs
    .readdirSync(sourceRecipeRoot, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".yaml.example"))
    .map((entry) => entry.name);

  for (const recipeFile of recipeFiles) {
    copyFile(
      path.join(sourceRecipeRoot, recipeFile),
      path.join(runtimeRecipeRoot, recipeFile.replace(/\.example$/, "")),
    );
  }

  return recipeFiles.map((name) => name.replace(/\.example$/, ""));
}

const syncedSkills = syncSkills();
const syncedRecipes = syncRecipes();

console.log(
  `Synced ${syncedSkills.length} skills into .agents/skills and ${syncedRecipes.length} recipes into .goose/recipes`,
);
