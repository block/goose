import { existsSync, readFileSync, statSync } from "node:fs";
import { execSync } from "node:child_process";
import { join } from "node:path";
import type { ValidationRule } from "./types.js";

export interface ValidationResult {
  passed: boolean;
  message?: string;
}

export function validateRule(
  rule: ValidationRule,
  workdir: string
): ValidationResult {
  switch (rule.type) {
    case "file_exists": {
      const fullPath = join(workdir, rule.path);
      const exists = existsSync(fullPath);
      return {
        passed: exists,
        message: exists ? undefined : `File not found: ${rule.path}`,
      };
    }

    case "file_not_empty": {
      const fullPath = join(workdir, rule.path);
      if (!existsSync(fullPath)) {
        return { passed: false, message: `File not found: ${rule.path}` };
      }
      const stat = statSync(fullPath);
      return {
        passed: stat.size > 0,
        message: stat.size > 0 ? undefined : `File is empty: ${rule.path}`,
      };
    }

    case "file_contains": {
      const fullPath = join(workdir, rule.path);
      if (!existsSync(fullPath)) {
        return { passed: false, message: `File not found: ${rule.path}` };
      }
      const content = readFileSync(fullPath, "utf-8");
      const contains = content.includes(rule.pattern);
      return {
        passed: contains,
        message: contains
          ? undefined
          : `File ${rule.path} does not contain: ${rule.pattern}`,
      };
    }

    case "file_matches": {
      const fullPath = join(workdir, rule.path);
      if (!existsSync(fullPath)) {
        return { passed: false, message: `File not found: ${rule.path}` };
      }
      const content = readFileSync(fullPath, "utf-8");
      const regex = new RegExp(rule.regex);
      const matches = regex.test(content);
      return {
        passed: matches,
        message: matches
          ? undefined
          : `File ${rule.path} does not match regex: ${rule.regex}`,
      };
    }

    case "file_not_matches": {
      const fullPath = join(workdir, rule.path);
      if (!existsSync(fullPath)) {
        return { passed: false, message: `File not found: ${rule.path}` };
      }
      const content = readFileSync(fullPath, "utf-8");
      const regex = new RegExp(rule.regex);
      const matches = regex.test(content);
      return {
        passed: !matches,
        message: !matches
          ? undefined
          : `File ${rule.path} should not match regex: ${rule.regex}`,
      };
    }

    case "command_succeeds": {
      try {
        execSync(rule.command, { cwd: workdir, stdio: "pipe" });
        return { passed: true };
      } catch (err) {
        return {
          passed: false,
          message: `Command failed: ${rule.command}`,
        };
      }
    }

    case "tool_called": {
      const logPath = join(workdir, "tool-calls.log");
      if (!existsSync(logPath)) {
        return { passed: false, message: "tool-calls.log not found" };
      }

      const content = readFileSync(logPath, "utf-8");
      const lines = content.trim().split("\n").filter(Boolean);

      // Find all calls to the specified tool
      const matchingCalls = lines
        .map((line) => {
          try {
            return JSON.parse(line);
          } catch {
            return null;
          }
        })
        .filter((entry) => entry?.tool === rule.tool);

      if (matchingCalls.length === 0) {
        return { passed: false, message: `Tool not called: ${rule.tool}` };
      }

      // If no arg requirements, just check tool was called
      if (!rule.args) {
        return { passed: true };
      }

      // Check if any call matches the arg requirements
      for (const call of matchingCalls) {
        const args = call.arguments || {};
        let allMatch = true;

        for (const [key, expected] of Object.entries(rule.args)) {
          const actual = args[key];
          if (actual === undefined) {
            allMatch = false;
            break;
          }

          // If expected starts/ends with /, treat as regex pattern
          if (typeof expected === "string" && expected.startsWith("/") && expected.endsWith("/")) {
            const pattern = new RegExp(expected.slice(1, -1), "i");
            if (!pattern.test(String(actual))) {
              allMatch = false;
              break;
            }
          } else {
            // Exact match (case-insensitive for strings)
            const actualStr = String(actual).toLowerCase();
            const expectedStr = String(expected).toLowerCase();
            if (!actualStr.includes(expectedStr)) {
              allMatch = false;
              break;
            }
          }
        }

        if (allMatch) {
          return { passed: true };
        }
      }

      return {
        passed: false,
        message: `Tool ${rule.tool} called but args didn't match: expected ${JSON.stringify(rule.args)}`,
      };
    }

    case "custom": {
      // Custom validators loaded dynamically
      return { passed: false, message: "Custom validators not yet implemented" };
    }

    default:
      return { passed: false, message: `Unknown rule type` };
  }
}

export function validateAll(
  rules: ValidationRule[],
  workdir: string
): Array<{ rule: ValidationRule; result: ValidationResult }> {
  return rules.map((rule) => ({
    rule,
    result: validateRule(rule, workdir),
  }));
}
