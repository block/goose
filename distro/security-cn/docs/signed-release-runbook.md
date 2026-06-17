# Security Goose signed release runbook

## Scope

这份 runbook 只覆盖：

- macOS `signed + notarized` 候选包演练
- CI preflight / bundle evidence 收集
- 安装后首启验收

这份 runbook 不覆盖：

- gateway
- LiteLLM
- AGS
- 在线 marketplace
- 企业后台

## Preconditions

执行者需要具备以下真实 Apple 发布条件：

- 当前候选分支已经推到目标 GitHub repo
- 目标 GitHub repo 已启用 Actions，并暴露 `Manual Desktop Bundle` / reusable macOS bundle workflows
- 目标 GitHub repo 暴露 `signing` environment
- GitHub `signing` environment 可读
- `APPLE_CERTIFICATE_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_TEAM_ID`
- `APPLE_ID`
- `APPLE_ID_PASSWORD`
- 有效的 `Developer ID Application` 证书
- 对应 Apple Developer 账号具备 notarization 权限

如果这些条件不完整，signed 演练应视为 Apple secrets / 证书 / 账号权限 blocker，而不是 Security Goose 代码 blocker。

在执行前，先跑一遍：

```bash
node scripts/check-security-github-release-readiness.mjs
```

这条检查会先确认：

- 当前分支是否已推到目标 repo
- 目标 repo 是否真的暴露所需 workflow
- `signing` environment 是否存在
- 当前 `gh` 身份是否具备 workflow 操作权限

如果执行者需要一份“可直接照着执行”的 secrets 注入、workflow 触发、artifact 下载和 Go/No-Go 模板，请同时打开：

- [`signed-release-handoff-panel.md`](./signed-release-handoff-panel.md)

## Recommended CI path

优先使用手工 workflow，而不是先推正式 release tag：

1. 打开 GitHub Actions 里的 `Manual Desktop Bundle`。
2. `branch` 选择待演练分支或 release candidate SHA 对应分支。
3. `signing` 设为 `true`。
4. `environment` 设为 `signing`。
5. 同时观察 arm64 与 x64 两个 macOS bundle job。

这样做的目的：

- 复用 Goose 现有 `bundle-desktop*.yml`
- 不新增发布平台层
- 不需要先触发正式 `release.yml`
- 可以把 signed rehearsal 和正式 tag release 明确分成两步

## Expected CI evidence

每个 macOS job 现在都会产出一份独立 artifact：

- `Security-Goose-macos-release-evidence-arm64`
- `Security-Goose-macos-release-evidence-x64`

artifact 内至少应包含：

- `signing-preflight.txt`
- `bundle-check.txt`
- `summary.json`
- `summary.md`

同时 job summary 也会渲染同一份 `summary.md` 内容。

## Signed rehearsal success shape

对 arm64 或 x64，每份 evidence 都应满足：

- `requested_mode=signed`
- `signing_requested=yes`
- `ready_for_signed_release=yes`
- `bundle_check=ok`
- `codesign_team` 不是 `not set`
- `spctl` 包含 `accepted`
- `stapler` 包含 `The validate action worked.`

只要其中任一项不满足，就不要把结果当成“已可正式签名发布”。

## Failure branches

### Preflight fails before bundle

常见表现：

- `missing_secrets=...`
- `invalid_secrets=...`
- `ready_for_signed_release=no`

结论：

- 这是 Apple secrets / 证书输入 blocker
- 先修 GitHub `signing` environment 或本地导出的 secrets

### GitHub workflow path is not ready

常见表现：

- `branch_on_target_repo=no`
- `required_workflow_files_present=no`
- `required_workflows_registered=no`
- `signing_environment_present=no`

结论：

- 这不是 Goose 产品代码 blocker
- 而是 signed rehearsal 目标 repo 还没有准备好

最小修正顺序：

1. 把当前候选分支推到目标 repo。
2. 确认目标 repo 默认分支或目标 ref 已包含：
   - `bundle-desktop-manual.yml`
   - `bundle-desktop.yml`
   - `bundle-desktop-intel.yml`
3. 确认目标 repo 已配置 `signing` environment。
4. 再重新执行 `node scripts/check-security-github-release-readiness.mjs`。

### Bundle check fails

常见表现：

- `codesign_team=not set`
- `spctl` 不含 `accepted`
- `stapler` 不含 `The validate action worked.`

结论：

- `codesign_team=not set` 说明没有真正走到 signed build 边界
- `spctl` / `stapler` 失败说明 notarization 或 Apple 服务链路没有闭环

### Build succeeds but install launch is blocked

先记录：

- `xattr -l "/path/to/Security Goose.app"`
- `spctl -a -vv "/path/to/Security Goose.app"`
- `xcrun stapler validate "/path/to/Security Goose.app"`

如果候选包是从 CI artifact、聊天工具或网盘下载的 zip，可能带有 `com.apple.quarantine`。这时要先记录 quarantine 状态，再决定是否临时清除它做安装验收：

```bash
xattr -dr com.apple.quarantine "/path/to/Security Goose.app"
```

这个动作只用于安装态排障，不是对 Goose runtime 的架构改造。

## Local signed rehearsal commands

如果执行者已经在本地拿到了 Apple 条件，可以按下面顺序做一次同构演练：

```bash
source bin/activate-hermit
export GOOSE_DESKTOP_SIGN=true
export APPLE_CERTIFICATE_BASE64=...
export APPLE_CERTIFICATE_PASSWORD=...
export APPLE_TEAM_ID=...
export APPLE_ID=...
export APPLE_ID_PASSWORD=...

node scripts/check-security-apple-signing-env.mjs --require-signed
pnpm --dir ui/desktop run bundle:default
./scripts/check-security-macos-bundle.sh --arch arm64 --expect signed --require-notarized
```

如果需要把输出沉淀成和 CI 一样的证据文件：

```bash
EVIDENCE_DIR="${TMPDIR:-/tmp}/security-goose-release-evidence-arm64"
mkdir -p "$EVIDENCE_DIR"
node scripts/check-security-apple-signing-env.mjs --require-signed 2>&1 | tee "$EVIDENCE_DIR/signing-preflight.txt"
./scripts/check-security-macos-bundle.sh --arch arm64 --expect signed --require-notarized 2>&1 | tee "$EVIDENCE_DIR/bundle-check.txt"
node scripts/render-security-macos-release-evidence.mjs \
  --arch arm64 \
  --expected-mode signed \
  --evidence-dir "$EVIDENCE_DIR"
```

## Signed candidate install acceptance checklist

对 signed 候选包，安装验收至少确认一次：

1. `xattr -l` 结果已记录；若存在 quarantine，已记录是否需要临时清除。
2. Finder / Launchpad 里 app 名称显示为 `Security Goose`。
3. 应用图标不是默认 Electron 占位图标。
4. 首次启动可进入 Security Goose 主界面，而不是 bare Electron `default_app`。
5. 默认语言是 `zh-CN` 口径。
6. Settings 里的 provider / model 默认接线符合 `distro/security-cn/config/desktop-env.example` 与 `model-catalog.json`。
7. 任务模板页“已保存任务模板”列表仍可见 6 个内置安全任务模板。
8. `漏洞研判`、`告警分析`、`IOC 研判`、`网页调查`、`报告生成`、`业务逻辑排查（WooYun-style）` 都仍走 recipe-backed 路径。
10. Extensions 视图里的推荐安全扩展状态没有回退。

## Release tag path

只有在手工 signed 演练已经通过后，才建议触发 `release.yml` 的 tag 路径：

1. 复用同一组 `signing` environment secrets。
2. 确认 `Manual Desktop Bundle` 的 signed 演练 artifact 已满足 success shape。
3. 再推 `v1.*` tag 进入正式 release workflow。

否则 release tag 失败时，很难区分是发布条件不足，还是版本候选本身有问题。
