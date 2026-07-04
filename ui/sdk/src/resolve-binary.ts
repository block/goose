import { createRequire } from "node:module";
import { dirname, join } from "node:path";

const PLATFORMS: Record<string, string> = {
  "darwin-arm64": "@repo-makeover/gosling-binary-darwin-arm64",
  "darwin-x64": "@repo-makeover/gosling-binary-darwin-x64",
  "linux-arm64": "@repo-makeover/gosling-binary-linux-arm64",
  "linux-x64": "@repo-makeover/gosling-binary-linux-x64",
  "win32-x64": "@repo-makeover/gosling-binary-win32-x64",
};

/**
 * Resolves the path to the gosling binary.
 *
 * Resolution order:
 *   1. `GOSLING_BINARY` environment variable (explicit override)
 *   2. Platform-specific `@repo-makeover/gosling-binary-*` optional dependency
 *
 * @throws if no binary can be found
 */
export function resolveGoslingBinary(): string {
  const envBinary = process.env.GOSLING_BINARY;
  if (envBinary) return envBinary;

  const key = `${process.platform}-${process.arch}`;
  const pkg = PLATFORMS[key];
  if (!pkg) {
    throw new Error(
      `No gosling binary available for ${key}. Set GOSLING_BINARY to the path of a gosling binary.`,
    );
  }

  try {
    const require = createRequire(import.meta.url);
    const pkgDir = dirname(require.resolve(`${pkg}/package.json`));
    const binName = process.platform === "win32" ? "gosling.exe" : "gosling";
    return join(pkgDir, "bin", binName);
  } catch {
    throw new Error(
      `gosling binary package ${pkg} is not installed. Set GOSLING_BINARY or install the native package.`,
    );
  }
}
