//! Supply-chain inspector: flags likely-malicious package installs before they run.
//!
//! The other inspectors look at *how* a command behaves (injection, egress). This
//! one looks at *what package it pulls in*. When the agent issues a shell command
//! that installs or fetch-executes an npm package (`npm`/`yarn`/`pnpm`/`bun`
//! install/add, or the `npx`/`bunx`/`pnpm dlx`/`yarn dlx`/`bun x` runners), it
//! extracts the target package name(s) and checks them against a deterministic,
//! offline typosquat heuristic: a name one edit away from a well-known package is
//! a classic supply-chain attack (`lodahs` for `lodash`, `exprcss` for `express`).
//! A match returns `RequireApproval` so the user can confirm before installing.
//!
//! Everything here is pure and offline: no network call, no latency, and it
//! complements the pattern/ML-based scanners rather than overlapping them.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use uuid::Uuid;

use crate::config::GooseMode;
use crate::conversation::message::{Message, ToolRequest};
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};

/// Well-known npm package names. These are the ones that show up again and again
/// as typosquat targets in real supply-chain incidents. Not a full top-1000; a
/// curated, high-signal set keeps false positives low.
const POPULAR_NPM_NAMES: &[&str] = &[
    "react",
    "react-dom",
    "vue",
    "angular",
    "lodash",
    "axios",
    "express",
    "next",
    "tailwindcss",
    "typescript",
    "vite",
    "webpack",
    "eslint",
    "prettier",
    "jest",
    "mocha",
    "chalk",
    "commander",
    "chokidar",
    "moment",
    "dayjs",
    "uuid",
    "yargs",
    "zod",
    "rxjs",
    "ramda",
    "dotenv",
    "cors",
    "body-parser",
    "socket.io",
    "mongoose",
    "sequelize",
    "prisma",
    "redux",
    "react-router",
    "react-router-dom",
    "react-query",
    "framer-motion",
    "styled-components",
    "antd",
    "bootstrap",
    "jquery",
    "underscore",
    "request",
    "node-fetch",
    "got",
    "puppeteer",
    "playwright",
    "cheerio",
    "fs-extra",
    "minimatch",
    "rimraf",
    "semver",
    "debug",
    "winston",
    "pino",
    "morgan",
    "helmet",
    "passport",
    "bcrypt",
    "jsonwebtoken",
    "argon2",
    "mysql2",
    "redis",
    "ioredis",
    "nodemon",
    "concurrently",
];

pub struct SupplyChainInspector;

impl SupplyChainInspector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SupplyChainInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolInspector for SupplyChainInspector {
    fn name(&self) -> &'static str {
        "supply_chain"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn inspect(
        &self,
        _session_id: &str,
        tool_requests: &[ToolRequest],
        _messages: &[Message],
        _goose_mode: GooseMode,
    ) -> Result<Vec<InspectionResult>> {
        let mut results = Vec::new();

        for tool_request in tool_requests {
            let tool_call = match &tool_request.tool_call {
                Ok(tc) => tc,
                Err(_) => continue,
            };
            if !is_shell_tool(tool_call.name.as_ref()) {
                continue;
            }
            let command = match extract_command(tool_call) {
                Some(c) => c,
                None => continue,
            };

            let mut flagged: Vec<(String, &'static str)> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();
            for spec in extract_install_targets(&command) {
                let name = package_name(&spec).to_ascii_lowercase();
                if let Some(popular) = typosquat_of(&name) {
                    if seen.insert(name.clone()) {
                        flagged.push((name, popular));
                    }
                }
            }
            if flagged.is_empty() {
                continue;
            }

            let detail = flagged
                .iter()
                .map(|(name, popular)| {
                    format!(
                        "`{}` is one edit away from the popular package `{}`",
                        name, popular
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            let finding_id = format!("SUPPLY-{}", Uuid::new_v4().simple());

            tracing::warn!(
                monotonic_counter.goose.supply_chain_typosquat_finding = 1,
                tool_name = tool_call.name.as_ref(),
                tool_request_id = %tool_request.id,
                finding_id = %finding_id,
                detail = %detail,
                "supply-chain: likely typosquat package install flagged"
            );

            results.push(InspectionResult {
                tool_request_id: tool_request.id.clone(),
                action: InspectionAction::RequireApproval(Some(format!(
                    "🔒 Possible malicious package install (typosquat)\n\n\
                    {}\n\n\
                    Typosquatting a popular package name is a common way to slip malware \
                    into a project. Confirm this is the package you intended before installing.\n\n\
                    Finding ID: {}",
                    detail, finding_id
                ))),
                reason: detail,
                // A single edit from a well-known name is a strong, but not certain,
                // signal, so we ask the user rather than hard-denying.
                confidence: 0.85,
                inspector_name: self.name().to_string(),
                finding_id: Some(finding_id),
            });
        }

        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Tool-call helpers (mirrors egress_inspector's surface)
// ---------------------------------------------------------------------------

fn is_shell_tool(name: &str) -> bool {
    matches!(
        name,
        "shell" | "bash" | "execute_command" | "run_command" | "terminal"
    ) || name.ends_with("__shell")
        || name.ends_with("__bash")
        || name.ends_with("__terminal")
}

fn extract_command(tool_call: &rmcp::model::CallToolRequestParams) -> Option<String> {
    let args = tool_call.arguments.as_ref()?;
    ["command", "cmd", "script", "input"]
        .iter()
        .find_map(|k| args.get(*k).and_then(|v| v.as_str()).map(|s| s.to_string()))
}

// ---------------------------------------------------------------------------
// Command parsing: extract package-install / runner targets
//
// Pure, no I/O. Ported from npmguard's install-command parser. Recognises
// npm/yarn/pnpm/bun install and the npx/bunx/dlx/`bun x` runners, and returns
// the target package spec(s). Fail-open: anything it does not understand yields
// nothing, so unrelated commands are never flagged.
// ---------------------------------------------------------------------------

fn extract_install_targets(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    for fragment in split_shell_fragments(command) {
        out.extend(packages_from_fragment(fragment.trim()));
    }
    out
}

/// Split a shell command into the separate commands it chains, so each is
/// inspected on its own. Separators: newline, `;`, `&&`, `||`, a single `|`
/// (pipeline) and a single `&` (background). Agents routinely batch installs
/// with newlines or `&&` (`cd ui\nnpm install evil`), and a separator-blind
/// parser would only ever see the first command's executable.
fn split_shell_fragments(cmd: &str) -> Vec<&str> {
    let bytes = cmd.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    // `cmd.get(range)` instead of `&cmd[range]`: all split points are ASCII
    // (`&`, `;`, `|`, `\n`) so the ranges are always valid, but `get` keeps
    // clippy's `string_slice` lint satisfied and can never panic.
    while i < bytes.len() {
        // Two-char operators first, so `&&`/`||` aren't mis-read as `&`/`|`.
        let two_char_op = i + 1 < bytes.len()
            && ((bytes[i] == b'&' && bytes[i + 1] == b'&')
                || (bytes[i] == b'|' && bytes[i + 1] == b'|'));
        if two_char_op {
            parts.push(cmd.get(start..i).unwrap_or_default());
            i += 2;
            start = i;
            continue;
        }
        if matches!(bytes[i], b';' | b'|' | b'&' | b'\n') {
            parts.push(cmd.get(start..i).unwrap_or_default());
            i += 1;
            start = i;
            continue;
        }
        i += 1;
    }
    parts.push(cmd.get(start..).unwrap_or_default());
    parts
}

/// Command wrappers that prefix the real command (`sudo npm i evil`,
/// `env FOO=bar npm i evil`); skipped so the wrapped install is still seen.
const COMMAND_WRAPPERS: &[&str] = &[
    "sudo", "doas", "env", "time", "nice", "nohup", "stdbuf", "command", "xargs",
];

fn packages_from_fragment(fragment: &str) -> Vec<String> {
    // Unquote each token (`"lodash"` -> `lodash`) and drop leading shell noise
    // (env assignments, wrapper commands) so the real binary is at the front.
    let tokens: Vec<&str> = fragment.split_whitespace().map(unquote).collect();
    let tokens = strip_command_prefixes(&tokens);
    let Some(&first) = tokens.first() else {
        return vec![];
    };
    // Strip any path prefix on the binary (`/usr/bin/npm` -> `npm`).
    let bin_base = first.rsplit('/').next().unwrap_or(first);
    match bin_base {
        "npm" | "npm.cmd" => parse_npm(tokens),
        "yarn" => parse_yarn(tokens),
        "pnpm" => parse_pnpm(tokens),
        "bun" => parse_bun(tokens),
        "npx" | "npx.cmd" | "bunx" | "bunx.cmd" => parse_runner(&tokens[1..]),
        _ => vec![],
    }
}

/// Strip one matched pair of surrounding quotes from a token
/// (`"lodash"` / `'lodash'` -> `lodash`). The shell tool passes the raw command
/// string, so quotes survive into tokens and would otherwise hide the name.
fn unquote(tok: &str) -> &str {
    let b = tok.as_bytes();
    if b.len() >= 2 && (b[0] == b'"' || b[0] == b'\'') && b[b.len() - 1] == b[0] {
        tok.get(1..b.len() - 1).unwrap_or(tok)
    } else {
        tok
    }
}

/// Skip leading shell noise that hides the real command: environment-variable
/// assignments (`FOO=bar`) and wrapper commands (`sudo`, `env`, ...) plus their
/// own options. Returns the slice starting at the wrapped binary, so a typosquat
/// install behind `sudo`/`env`/a var assignment is still inspected.
fn strip_command_prefixes<'s, 'a>(tokens: &'a [&'s str]) -> &'a [&'s str] {
    let mut i = 0;
    loop {
        while i < tokens.len() && is_env_assignment(tokens[i]) {
            i += 1;
        }
        if i < tokens.len() && is_command_wrapper(tokens[i]) {
            i += 1;
            while i < tokens.len() && tokens[i].starts_with('-') {
                i += 1;
            }
            continue;
        }
        break;
    }
    tokens.get(i..).unwrap_or(&[])
}

/// A `KEY=VALUE` shell assignment (`NODE_ENV=production`): a shell identifier
/// followed by `=`. Such a token is never a binary name.
fn is_env_assignment(tok: &str) -> bool {
    let Some(eq) = tok.find('=') else {
        return false;
    };
    eq > 0
        && tok
            .as_bytes()
            .iter()
            .take(eq)
            .enumerate()
            .all(|(i, &b)| b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit()))
}

fn is_command_wrapper(tok: &str) -> bool {
    let base = tok.rsplit('/').next().unwrap_or(tok);
    COMMAND_WRAPPERS.contains(&base)
}

/// Locate the package-manager subcommand, skipping any leading global options.
///
/// npm/yarn/pnpm/bun all accept global flags before the subcommand
/// (`npm --prefix ui install x`), so it isn't always `tokens[1]`. We skip leading
/// options: `--flag=value` is self-contained, and a bare `--flag` consumes the
/// next token as its value unless that token starts with `-` or is itself a
/// subcommand. The first non-option token is the subcommand, or there is none.
/// Stopping at the first non-option token keeps `npm run add x` (a script named
/// `add`) from being misread as an install.
fn subcommand_index(tokens: &[&str], subcommands: &[&str]) -> Option<usize> {
    let is_sub = |t: &str| subcommands.iter().any(|s| s.eq_ignore_ascii_case(t));
    let mut i = 1;
    while i < tokens.len() {
        let tok = tokens[i];
        if !tok.starts_with('-') {
            return is_sub(tok).then_some(i);
        }
        let consumes_value = !tok.contains('=')
            && tokens
                .get(i + 1)
                .is_some_and(|n| !n.starts_with('-') && !is_sub(n));
        i += if consumes_value { 2 } else { 1 };
    }
    None
}

fn parse_npm(tokens: &[&str]) -> Vec<String> {
    let Some(idx) = subcommand_index(tokens, &["install", "i", "add", "exec", "x"]) else {
        return vec![];
    };
    match tokens[idx].to_ascii_lowercase().as_str() {
        "install" | "i" | "add" => collect_pkg_args(&tokens[idx + 1..]),
        "exec" | "x" => parse_runner(&tokens[idx + 1..]),
        _ => vec![],
    }
}

fn parse_yarn(tokens: &[&str]) -> Vec<String> {
    let Some(idx) = subcommand_index(tokens, &["add", "dlx"]) else {
        return vec![];
    };
    match tokens[idx].to_ascii_lowercase().as_str() {
        "add" => collect_pkg_args(&tokens[idx + 1..]),
        "dlx" => parse_runner(&tokens[idx + 1..]),
        _ => vec![],
    }
}

fn parse_pnpm(tokens: &[&str]) -> Vec<String> {
    let Some(idx) = subcommand_index(tokens, &["add", "install", "i", "dlx"]) else {
        return vec![];
    };
    match tokens[idx].to_ascii_lowercase().as_str() {
        "add" | "install" | "i" => collect_pkg_args(&tokens[idx + 1..]),
        "dlx" => parse_runner(&tokens[idx + 1..]),
        _ => vec![],
    }
}

fn parse_bun(tokens: &[&str]) -> Vec<String> {
    let Some(idx) = subcommand_index(tokens, &["add", "install", "i", "x"]) else {
        return vec![];
    };
    match tokens[idx].to_ascii_lowercase().as_str() {
        "add" | "install" | "i" => collect_pkg_args(&tokens[idx + 1..]),
        "x" => parse_runner(&tokens[idx + 1..]),
        _ => vec![],
    }
}

/// A runner fetches and executes one package on the fly, so only the *executed*
/// package is the install target, not the arguments passed to it
/// (`npx create-react-app my-app` -> `create-react-app`, never `my-app`).
fn parse_runner(args: &[&str]) -> Vec<String> {
    let mut packages = Vec::new();
    let mut saw_explicit_package = false;
    let mut i = 0usize;
    while i < args.len() {
        let arg = args[i];
        if arg == "-p" || arg == "--package" {
            if let Some(&val) = args.get(i + 1) {
                if looks_like_package_spec(val) {
                    packages.push(val.to_string());
                    saw_explicit_package = true;
                }
            }
            i += 2;
            continue;
        }
        if let Some(val) = arg.strip_prefix("--package=") {
            if looks_like_package_spec(val) {
                packages.push(val.to_string());
                saw_explicit_package = true;
            }
            i += 1;
            continue;
        }
        if arg.starts_with('-') {
            i += 1;
            continue;
        }
        if !saw_explicit_package && looks_like_package_spec(arg) {
            packages.push(arg.to_string());
        }
        // Everything after the executed token is its arguments: stop.
        break;
    }
    packages
}

/// npm/yarn/pnpm/bun options that take a separate value token (`--prefix ui`),
/// so the value isn't mistaken for a package. Boolean flags (`--save-dev`, `-D`)
/// are intentionally absent: the token after them IS a package.
const VALUE_FLAGS: &[&str] = &[
    "--prefix",
    "-C",
    "--registry",
    "--cache",
    "--userconfig",
    "--globalconfig",
    "--workspace",
    "-w",
    "--omit",
    "--include",
    "--save-prefix",
    "--loglevel",
    "--filter",
    "--dir",
];

fn collect_pkg_args(args: &[&str]) -> Vec<String> {
    let mut packages = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i];
        if arg.starts_with('-') {
            // A value-taking flag (`--prefix ui`) consumes its value, so we don't
            // mistake the value (`ui`) for a package name.
            i += if !arg.contains('=') && VALUE_FLAGS.contains(&arg) {
                2
            } else {
                1
            };
            continue;
        }
        if !matches!(arg, ">" | ">>" | "<") && looks_like_package_spec(arg) {
            packages.push(arg.to_string());
        }
        i += 1;
    }
    packages
}

fn looks_like_package_spec(s: &str) -> bool {
    let Some(first) = s.bytes().next() else {
        return false;
    };
    if first == b'@' {
        return s.contains('/');
    }
    first.is_ascii_alphanumeric() || first == b'_'
}

/// Strip a `@version` suffix to get the bare package name. Handles scoped names
/// (`@scope/pkg@1.2.3` -> `@scope/pkg`, `@scope/pkg` -> `@scope/pkg`).
fn package_name(spec: &str) -> &str {
    match spec.rsplit_once('@') {
        // No `@`, or the only `@` is the leading scope marker: whole spec.
        None | Some(("", _)) => spec,
        Some((name, _version)) => name,
    }
}

// ---------------------------------------------------------------------------
// Typosquat heuristic
// ---------------------------------------------------------------------------

/// If `name` is exactly one edit away from a well-known package (and not a
/// well-known package itself), return that package. One edit = substitution,
/// insertion, deletion, or an adjacent transposition (optimal string alignment).
fn typosquat_of(name: &str) -> Option<&'static str> {
    if POPULAR_NPM_NAMES
        .iter()
        .any(|n| n.eq_ignore_ascii_case(name))
    {
        return None;
    }
    let mut best: Option<(&'static str, usize)> = None;
    for &candidate in POPULAR_NPM_NAMES {
        let dist = osa_distance(name, candidate);
        if dist == 0 {
            continue;
        }
        if best.map(|(_, bd)| dist < bd).unwrap_or(true) {
            best = Some((candidate, dist));
        }
    }
    // Require the popular name to be long enough that a single random edit is
    // unlikely to collide, matching npmguard's heuristic. POPULAR_NPM_NAMES are
    // all ASCII, so byte-len here equals char-len.
    match best {
        Some((candidate, 1)) if candidate.len() > 4 => Some(candidate),
        _ => None,
    }
}

/// Optimal string alignment (restricted Damerau-Levenshtein) distance. For the
/// distance-of-1 check we use it for, this is identical to full Damerau, and it
/// needs no extra dependency.
fn osa_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut val = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                val = val.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = val;
        }
    }
    d[n][m]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolRequestParams;
    use rmcp::object;

    fn shell_request(id: &str, command: &str) -> ToolRequest {
        ToolRequest {
            id: id.to_string(),
            tool_call: Ok(
                CallToolRequestParams::new("shell").with_arguments(object!({ "command": command }))
            ),
            metadata: None,
            tool_meta: None,
        }
    }

    async fn inspect(command: &str) -> Vec<InspectionResult> {
        SupplyChainInspector::new()
            .inspect(
                "test",
                &[shell_request("req", command)],
                &[],
                GooseMode::Approve,
            )
            .await
            .unwrap()
    }

    // --- parser ---

    #[test]
    fn parses_npm_install_target() {
        assert_eq!(
            extract_install_targets("npm install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(extract_install_targets("npm i -D lodahs"), vec!["lodahs"]);
        assert_eq!(
            extract_install_targets("cd x && npm install evil"),
            vec!["evil"]
        );
    }

    #[test]
    fn parses_runner_executed_package_only() {
        // The executed package is checked, the argument to it is not.
        assert_eq!(
            extract_install_targets("npx create-react-app my-app"),
            vec!["create-react-app"]
        );
        assert_eq!(
            extract_install_targets("bunx prettier --write ."),
            vec!["prettier"]
        );
        assert_eq!(
            extract_install_targets("pnpm dlx create-vite app"),
            vec!["create-vite"]
        );
    }

    #[test]
    fn ignores_non_install_commands() {
        assert!(extract_install_targets("ls -la").is_empty());
        assert!(extract_install_targets("npm run build").is_empty());
        assert!(extract_install_targets("npm install").is_empty());
    }

    #[test]
    fn parses_install_batched_across_newlines() {
        // Agents commonly batch commands with newlines; each line is its own
        // command, so the install must still be seen.
        assert_eq!(
            extract_install_targets("cd ui\nnpm install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("export X=1\nyarn add exprcss\necho done"),
            vec!["exprcss"]
        );
        // `||` and `&` also separate commands.
        assert_eq!(
            extract_install_targets("test -d ui || npm install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("npm install lodahs & echo bg"),
            vec!["lodahs"]
        );
    }

    #[test]
    fn parses_install_with_global_options_before_subcommand() {
        // npm/pnpm accept global flags (some taking a value) before the
        // subcommand; the install target must not slip past because of them.
        assert_eq!(
            extract_install_targets("npm --prefix ui install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("npm --prefix=ui install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("npm -g install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("pnpm --dir ui add exprcss"),
            vec!["exprcss"]
        );
    }

    #[test]
    fn run_scripts_named_like_subcommands_are_not_installs() {
        // `npm run add x` runs a user script called "add"; the option-skipping
        // parser must stop at the first positional token (`run`) and not treat a
        // later `add`/`install` keyword as the subcommand.
        assert!(extract_install_targets("npm run add lodahs").is_empty());
        assert!(extract_install_targets("npm run install").is_empty());
    }

    #[test]
    fn strips_quotes_around_package_and_binary() {
        assert_eq!(
            extract_install_targets("npm install \"lodahs\""),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("yarn add 'exprcss'"),
            vec!["exprcss"]
        );
        assert_eq!(
            extract_install_targets("npx \"expresss\""),
            vec!["expresss"]
        );
    }

    #[test]
    fn parses_install_behind_env_and_wrappers() {
        // Env-var assignment prefixes and wrapper commands must not hide the
        // install from inspection.
        assert_eq!(
            extract_install_targets("FOO=bar npm install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("NODE_ENV=production npm i exprcss"),
            vec!["exprcss"]
        );
        assert_eq!(
            extract_install_targets("sudo npm install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("sudo -H npm install lodahs"),
            vec!["lodahs"]
        );
        assert_eq!(
            extract_install_targets("env FOO=bar npm install lodahs"),
            vec!["lodahs"]
        );
    }

    #[test]
    fn flag_values_are_not_treated_as_packages() {
        // `axio` is the value of `--prefix`, not a package; it must not be
        // collected even though it is one edit from `axios`.
        assert_eq!(
            extract_install_targets("npm install --prefix axio lodash"),
            vec!["lodash"]
        );
        assert_eq!(
            extract_install_targets("pnpm add --filter prism react"),
            vec!["react"]
        );
        // Boolean flags still let the following token be a package.
        assert_eq!(
            extract_install_targets("npm install --save-dev lodahs react"),
            vec!["lodahs", "react"]
        );
    }

    #[test]
    fn package_name_strips_version_and_keeps_scope() {
        assert_eq!(package_name("lodash@4.17.21"), "lodash");
        assert_eq!(package_name("@scope/pkg@1.2.3"), "@scope/pkg");
        assert_eq!(package_name("react"), "react");
    }

    // --- typosquat ---

    #[test]
    fn typosquat_detects_one_edit_names() {
        assert_eq!(typosquat_of("lodahs"), Some("lodash")); // transposition
        assert_eq!(typosquat_of("expresss"), Some("express")); // insertion
        assert_eq!(typosquat_of("axfos"), Some("axios")); // substitution
    }

    #[test]
    fn typosquat_ignores_legitimate_and_unrelated_names() {
        assert_eq!(typosquat_of("lodash"), None); // the real one
        assert_eq!(typosquat_of("react"), None);
        assert_eq!(typosquat_of("zustand"), None); // unrelated
    }

    // --- inspector end to end ---

    #[tokio::test]
    async fn flags_typosquat_install_for_approval() {
        let results = inspect("npm install lodahs").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].inspector_name, "supply_chain");
        assert!(matches!(
            results[0].action,
            InspectionAction::RequireApproval(_)
        ));
        assert!(results[0].reason.contains("lodash"));
    }

    #[tokio::test]
    async fn allows_legitimate_install() {
        assert!(inspect("npm install lodash").await.is_empty());
        assert!(inspect("npm install react react-dom").await.is_empty());
    }

    #[tokio::test]
    async fn ignores_non_shell_and_non_install() {
        // Non-install shell command.
        assert!(inspect("git status").await.is_empty());
        // Non-shell tool.
        let req = ToolRequest {
            id: "r".into(),
            tool_call: Ok(CallToolRequestParams::new("text_editor")
                .with_arguments(object!({ "command": "npm install lodahs" }))),
            metadata: None,
            tool_meta: None,
        };
        let out = SupplyChainInspector::new()
            .inspect("t", &[req], &[], GooseMode::Approve)
            .await
            .unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn flags_typosquat_via_runner() {
        let results = inspect("npx expresss --version").await;
        assert_eq!(results.len(), 1);
        assert!(results[0].reason.contains("express"));
    }

    #[tokio::test]
    async fn flags_typosquat_install_batched_with_newline() {
        // End-to-end: a newline-batched install must still reach approval.
        let results = inspect("cd ui\nnpm install lodahs").await;
        assert_eq!(results.len(), 1);
        assert!(results[0].reason.contains("lodash"));
    }

    #[tokio::test]
    async fn flags_typosquat_behind_sudo_but_not_flag_value() {
        // A typosquat behind `sudo` still fires.
        let flagged = inspect("sudo npm install lodahs").await;
        assert_eq!(flagged.len(), 1);
        assert!(flagged[0].reason.contains("lodash"));
        // A benign `--prefix` value that merely looks typosquat-ish must not fire.
        assert!(inspect("npm install --prefix axio lodash").await.is_empty());
    }
}
