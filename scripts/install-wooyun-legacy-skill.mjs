import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const targetRoot = path.join(
  repoRoot,
  ".agents/skills/wooyun-legacy/external/upstream",
);
const requiredReferenceFiles = [
  "authentication-domain.md",
  "authorization-domain.md",
  "configuration-domain.md",
  "financial-domain.md",
  "information-domain.md",
  "logic-flow-domain.md",
];

function fail(message) {
  console.error(message);
  process.exit(1);
}

function copyFile(sourcePath, targetPath) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, fs.readFileSync(sourcePath, "utf8"));
}

function resolveSourceSkillRoot(inputPath) {
  const candidateRoots = [
    path.resolve(inputPath),
    path.resolve(inputPath, "plugins/wooyun-legacy/skills/wooyun-legacy"),
    path.resolve(inputPath, "skills/wooyun-legacy"),
  ];

  return candidateRoots.find((candidateRoot) =>
    fs.existsSync(path.join(candidateRoot, "SKILL.md")),
  );
}

const inputPath = process.argv[2];
if (!inputPath) {
  fail(
    "Usage: node scripts/install-wooyun-legacy-skill.mjs /path/to/wooyun-legacy-or-skill-dir",
  );
}

const sourceSkillRoot = resolveSourceSkillRoot(inputPath);
if (!sourceSkillRoot) {
  fail(
    "Could not find plugins/wooyun-legacy/skills/wooyun-legacy or skills/wooyun-legacy from the provided path.",
  );
}

const sourceRepoRoot = path.resolve(sourceSkillRoot, "../../../../");
const sourceSkillFile = path.join(sourceSkillRoot, "SKILL.md");
const sourceReadme = path.join(sourceRepoRoot, "README.md");
const sourceLicense = path.join(sourceRepoRoot, "LICENSE");

for (const referenceFile of requiredReferenceFiles) {
  const referencePath = path.join(sourceSkillRoot, "references", referenceFile);
  if (!fs.existsSync(referencePath)) {
    fail(`Missing required upstream reference file: ${referencePath}`);
  }
}

copyFile(sourceSkillFile, path.join(targetRoot, "UPSTREAM-SKILL.md"));

if (fs.existsSync(sourceReadme)) {
  copyFile(sourceReadme, path.join(targetRoot, "UPSTREAM-README.md"));
}

if (fs.existsSync(sourceLicense)) {
  copyFile(sourceLicense, path.join(targetRoot, "LICENSE"));
}

for (const referenceFile of requiredReferenceFiles) {
  copyFile(
    path.join(sourceSkillRoot, "references", referenceFile),
    path.join(targetRoot, "references", referenceFile),
  );
}

console.log(
  `Installed wooyun-legacy upstream reference pack into ${path.relative(repoRoot, targetRoot)}`,
);
