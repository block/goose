# 测试、CI/CD 与发布方案

## Goal 7 当前最小落地

当前 fork 不重写 upstream 的通用 `ci.yml` / release 体系，而是补一条独立的
`security-goose-v1a-checks.yml`，只覆盖 `macOS-only` 本机预览和仓库内自动校验。

共享检查入口：

- `scripts/check-security-v1a.sh`

当前 workflow 实际执行链路：

1. `CI=1 pnpm --dir ui install --frozen-lockfile --ignore-scripts`
2. `pnpm --dir ui --filter @aaif/goose-sdk run build`
3. `node scripts/sync-security-runtime-assets.mjs`
4. `node scripts/smoke-security-extensions.mjs`
5. Goal 3-8 相关 `vitest`
6. `pnpm --dir ui/desktop exec tsc --noEmit`
7. `pnpm --dir ui/desktop run lint:check`
8. `node scripts/validate-security-distro.mjs`
9. `git diff --check`

打包/install 边界的最小附加校验入口：

- `scripts/check-security-macos-bundle.sh`
- `scripts/check-security-apple-signing-env.mjs`
- `scripts/check-security-github-release-readiness.mjs`
- `scripts/render-security-macos-release-evidence.mjs`

这条脚本当前覆盖：

- `Info.plist` 中的 `product name / bundle id / signing mode`
- packaged `Contents/Resources/bin/goosed` 存在性
- packaged `Contents/Resources/security-cn` 存在性
- `codesign --verify --deep --strict`
- zip 解包后的 `.app` 存在性
- `spctl` 当前结果打印
- signed 模式下可选的 `stapler validate` 证据检查

`check-security-apple-signing-env.mjs` 当前覆盖：

- 是否请求 `local-preview` 还是 `signed`
- `APPLE_CERTIFICATE_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_TEAM_ID`
- `APPLE_ID`
- `APPLE_ID_PASSWORD`
- local-preview fallback 提示
- signed mode 下的缺失 secrets / 明显错误 secrets 提示

`check-security-github-release-readiness.mjs` 当前覆盖：

- 当前分支是否已推到目标 GitHub repo
- 目标 repo 是否暴露 `Manual Desktop Bundle`
- 目标 repo 是否暴露 arm64 / x64 macOS reusable bundle workflow
- 目标 repo 是否存在 `signing` environment
- 当前 `gh` 身份是否具备 workflow 操作权限

`render-security-macos-release-evidence.mjs` 当前覆盖：

- 从 `signing-preflight.txt` 与 `bundle-check.txt` 汇总一份 `summary.md`
- 渲染 `summary.json`
- 把 `preflight / codesign / spctl / stapler` 证据压成可上传 artifact 的固定结构
- 对 `signed` 和 `local-preview` 两种期望值给出不同 verdict

注意：

- 对 `local-preview` bundle，这条脚本不会把 `spctl rejected` 当成失败
- 因为没有 notarization secrets 的本地预览包，本来就不是正式 Apple 分发产物
- 这条边界要通过文档和脚本写清楚，而不是伪装成“已正式发布”

当前明确不纳入 Goal 7 门禁：

- gateway
- LiteLLM
- AGS
- 在线 marketplace
- 企业后台

## 测试原则

V1 的测试重点不是平台级极致完备，而是：

- 品牌发行版能启动
- 模型切换不坏
- 安全 skills 可加载
- MCP 扩展可启用
- 桌面打包能产出

## 测试层次

## 1. 配置校验

必须自动校验：

- `init-config.yaml`
- `desktop-env.example`
- `model-catalog.json`
- `feature-flags.json`
- `bundled-extensions.security.json`
- locale JSON
- recipe YAML

工具建议：

- JSON Schema
- YAML Schema
- 自定义 Node 校验脚本

## 2. Skills 测试

重点校验：

- 目录结构完整
- `SKILL.md` 有必要 frontmatter
- 支持文件路径有效
- 输出模板字段完整

V1 不要求做复杂语义基准测试，但至少要做结构和 smoke 测试。

## 3. MCP / 扩展测试

重点校验：

- 扩展清单能被 Goose 发现
- MCP wrapper 配置有效
- 至少 2 条 Goose-native 本地扩展链路可 smoke
- 失败时错误文案可理解

## 4. 桌面测试

### 单元 / 组件测试

- 关键设置页
- 模型切换入口
- 语言切换入口
- 技能/任务入口

### E2E 测试

至少覆盖：

1. 首次启动
2. 默认中文
3. 切换模型
4. 启用一个安全 skill/recipe
5. 发起一次漏洞研判任务

## 5. 手工验收脚本

每个 release 候选版本必须手工验证：

- macOS 安装启动
- 图标与名称正确
- 中文文案无明显漏翻
- 默认模型配置正确
- skills 可用
- MCP 可启

如果未来进入 `V1.5+`，再单独增加 gateway/LiteLLM 测试，不混进 V1a 主线门禁。

## GitHub Actions 设计

## `ci.yml`

upstream `ci.yml` 继续负责通用 Goose 主线检查。
V1a 当前不去改造它的全量矩阵，而是补充独立 workflow：

- `security-goose-v1a-checks.yml`

触发建议：

- `push` 到 `main` 与 `codex/*`
- `pull_request` 到 `main`
- `workflow_dispatch`

建议 job：

- `rust-check`
- `desktop-check`
- `config-check`
- `docs-check`

### `rust-check`

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`

### `desktop-check`

- `pnpm install`
- `pnpm lint`
- `pnpm test`
- `pnpm build`

### `config-check`

- 校验 skills / recipes / locales / extensions / model catalog

### `docs-check`

- markdown lint
- link check

## `release-desktop.yml`

在 tag 触发。

建议目标：

- macOS desktop bundle

V1a 当前先复用现有 desktop 打包脚本和 upstream reusable bundle workflow，
本阶段不新增独立发布平台层。

当前 macOS 打包边界分两类：

- `local-preview`
  - 入口：`pnpm --dir ui/desktop run bundle:default`
  - 会默认显式设置 `GOOSE_DESKTOP_SIGN=false`
  - 会关闭 Electron cookie encryption
  - 会在 app `LSEnvironment` 里写入 `GOOSE_DISABLE_KEYRING=1`
  - 会在 zip 前执行 ad-hoc re-sign，确保 `codesign --verify` 可通过
- `signed`
  - 入口：reusable `bundle-desktop*.yml` + `release.yml`
  - 也可先走 `bundle-desktop-manual.yml` 的 `signing=true` 演练路径
  - 依赖 `APPLE_TEAM_ID / APPLE_ID / APPLE_ID_PASSWORD / certificate secrets`
  - 保留 keychain 与 cookie encryption
  - reusable workflow 现在会先跑 signing preflight，再要求 bundle 提供 notarization 证据
  - arm64 / x64 workflow 现在都会上传 `Security-Goose-macos-release-evidence-*` artifact，并把 `summary.md` 写入 job summary
  - 当前是否真能通过，仍取决于 GitHub secrets / Apple 环境，不取决于 V1a 代码架构

产物：

- dmg / zip 或对应 macOS 包

## `docs.yml`

在文档变更时触发：

- markdown lint
- broken link check
- examples 存在性校验

## `upstream-sync-check.yml`

每周定时：

- 检查 upstream Goose 是否有新 release / main 差异
- 输出提醒，不自动合并

## 分支策略

- `main`：可发布
- `develop`：集成分支，可选
- `codex/*`：Codex 实现分支
- `feat/*`：功能分支
- `fix/*`：修复分支
- `docs/*`：文档分支

V1a 推荐最简方案：

- `main`
- `codex/*`

## 质量门禁

PR 合并前必须满足：

- Rust checks 通过
- Desktop checks 通过
- Config checks 通过
- 至少 1 次手工 smoke 说明

## 安全门禁

建议增加：

- `gitleaks` 或同类 secret scan
- `npm audit` / `cargo audit`

V1 不要把供应链安全留到最后。

## 发布节奏

建议：

- 每周 1 次内部测试版
- 每 2 周 1 次候选版
- V1 首发后再提高节奏

## 验收标准

- PR 有自动 CI
- tag 能产出桌面构建
- 配置与 skills 有自动校验
- 模型、任务入口、MCP 的关键链路被 smoke 覆盖
