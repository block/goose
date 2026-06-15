# security-cn operator guide

## Goal 7 当前边界

- 只覆盖 `macOS-only` 本机预览、仓库内自动校验和桌面本地打包
- 继续复用 Goose 原生 `recipe / skills / MCP / desktop shell` 入口
- 不引入并行 runtime、gateway、LiteLLM、AGS、在线 marketplace、企业后台

## 本机预览最小步骤

1. 激活仓库工具链：

   ```bash
   source bin/activate-hermit
   ```

2. 让 Goose core 和 desktop 默认配置对齐：

   ```bash
   cp distro/security-cn/config/init-config.yaml.example init-config.yaml
   set -a
   source distro/security-cn/config/desktop-env.example
   set +a
   ```

3. 同步安全运行时素材到 Goose 当前真实入口：

   ```bash
   node scripts/sync-security-runtime-assets.mjs
   ```

4. 安装 desktop 依赖并启动本机预览：

   ```bash
   CI=1 pnpm --dir ui install --frozen-lockfile --ignore-scripts
   pnpm --dir ui --filter @aaif/goose-sdk run build
   ./scripts/start-security-preview.sh
   ```

   这个入口会：

   - 先同步 `.agents/skills` 和 `.goose/recipes` 本地预览素材
   - 启动前只清理当前仓库这份 repo preview 的旧 Electron / `goosed` 进程，避免 single-instance 复用到失效窗口
   - 启动后会尝试把当前仓库这份 `Security Goose` Electron 窗口自动切到前台，方便 Computer Use 和人工可视回归
   - 只接受 repo 自己的 `goosed` 产物
   - macOS 本机预览默认优先使用 repo 自编的 `target/release/goosed`
   - 如果没有可用的 staged / release 产物，才会自动构建 repo 自己的 `goosed`
   - 把 desktop `userData` 默认隔离到 `<repo>/.preview/user-data`
   - 把默认工作目录固定到当前仓库根目录，避免 recipe/skill 入口误落到 `~/`

   如果你需要直接走 desktop 原生命令，也要带上仓库工作目录：

   ```bash
   pnpm --dir ui/desktop run start-gui -- --dir "$PWD"
   ```

   这条 direct `start-gui` 现在进入“开发态支持”范围，但仍然不是官方 preview 入口：

   - `--dir "$PWD"` 现在会稳定透传到 Electron 主进程，不再偶发落到 bare Electron `default_app`
   - direct 入口适合做 desktop 主进程排障、startup diagnostics 验证和本地开发态复现
   - 预期当前仓库已经先有 `target/release/goosed`
   - desktop 开发态现在会优先复用 `target/release/goosed`，其次 `target/debug/goosed`，最后才回退 `ui/desktop/src/bin/goosed`
   - 原因是当前 macOS 会话里，`ui/desktop/src/bin/goosed` 可能间歇性卡在 `_dyld_start`，表现为“进程存在但不监听端口”
   - 如果要复用和官方 wrapper 一样的 backend 与隔离路径，仍然要先导出 `scripts/launch-security-preview-backend.sh` 给出的环境变量，并显式设置 `GOOSE_USER_DATA_DIR`
   - 如果当前已经有 repo preview 的 Electron 在跑，再裸开 `start-gui`，你拿到的日志和窗口可能会混入旧进程状态；排障时要先停掉旧 repo preview

   如果需要沿用仓库现有整合启动链路，也可以执行：

   ```bash
   just run-security-preview
   ```

   当前这条链路默认只使用当前仓库的 `goosed`：

   - 官方 preview wrapper 优先复用 `target/release/goosed`
   - 其次才考虑 `target/debug/goosed`
   - `ui/desktop/src/bin/goosed` 只作为 fallback / packaging staging 副本
   - 明确拒绝把外部 `Goose.app` 自带的 `goosed` 注入到本仓库预览

   当前已知的 macOS 构建现状：

   - repo 自编 `target/release/goosed` 已验证可正常输出 fingerprint、监听端口并返回 `/status`
   - 当前会话里的 `target/debug/goosed` 可能卡在 `_dyld_start`，表现为“进程存在但不监听端口”
   - 这属于本地 debug 二进制可执行性 blocker，不是 desktop 页面逻辑 blocker
   - 如需强制切回 debug，可显式设置 `GOOSED_BUILD_PROFILE=debug`

   如果首次冷启动卡在 Rust crates 下载，而不是代码编译报错，可以只加一个临时镜像环境变量后重试：

   ```bash
   export GOOSE_CARGO_REGISTRY_MIRROR=rsproxy-cn
   ./scripts/start-security-preview.sh
   ```

   这个 fallback 只影响 `cargo build -p goose-server --bin goosed` 的下载源选择，不会引入新的运行时层，也不会修改仓库默认 Cargo 配置。

   如果当前桌面会话里还有 Codex、Chrome 或其他窗口抢前台，也可以手工再次聚焦 repo 预览：

   ```bash
   ./scripts/focus-security-preview-window.sh
   ```

   这个聚焦脚本现在只会提升“当前仓库这份 preview Electron 进程”的窗口，不再直接激活通用 `Electron.app`。

## Goal 7 自动校验入口

仓库内最小校验链路统一走：

```bash
./scripts/check-security-v1a.sh
```

这个脚本会顺序执行：

- `CI=1 pnpm --dir ui install --frozen-lockfile --ignore-scripts`
- `pnpm --dir ui --filter @aaif/goose-sdk run build`
- `node scripts/sync-security-runtime-assets.mjs`
- `node ui/desktop/scripts/ensure-goosed-dev.js`
- `node scripts/smoke-security-extensions.mjs`
- Goal 3-6 相关 `vitest`
- `pnpm --dir ui/desktop exec tsc --noEmit`
- `pnpm --dir ui/desktop run lint:check`
- `node scripts/validate-security-distro.mjs`
- `git diff --check`

## Goal 8 扩展 smoke 检查表

仓库内自动 smoke 已覆盖：

- `browser-assist-mcp`
  - `initialize`
  - `tools/list`
  - `summarize_web_page`
  - `extract_page_observables`
- `threat-intel-mcp`
  - `initialize`
  - `tools/list`
  - `extract_observables_from_text`
  - `analyze_observable`
  - `enrich_domain_dns`

当前仍是 disabled / blocker：

- `aiseesec-mcp`
  - 需要外部专有 API / 账号
- `local-security-gateway-mcp`
  - Goal 8 不实现 gateway

## Goal 6 安全入口 smoke 检查表

完成上面的本机预览后，至少手工确认一次：

1. Launcher 快速入口可见 6 个安全任务：
   - 漏洞研判
   - 告警分析
   - IOC 研判
   - 网页调查
   - 报告生成
   - 业务逻辑排查（WooYun-style）
2. Recipes 页顶部安全任务卡片区可见 6 个入口。
3. `security-vuln-triage`、`alert-investigation`、`web-investigation` 显示为 recipe-backed。
4. `ioc-analysis`、`report-writing`、`wooyun-legacy` 显示为 guided preview chat。
5. 点击“漏洞研判”或“告警分析”时，会沿 Goose 现有 `recipeId` 路径启动。

## Goal 9 任务与扩展联动 smoke 检查表

继续至少手工确认一次：

1. Launcher 安全任务区可见扩展状态总览。
   - `Browser Assist` / `Threat Intel` 标记为 `Local preview`
   - `AiseeSec` 标记为 `Blocked`
   - `Security Gateway` 标记为 `Disabled stub`
2. Launcher 与 Recipes 安全任务卡片可见 `Recommended extensions` 标签区。
3. Recipes 页安全任务区可见 `Open Extensions` 按钮。
4. 点击 `Open Extensions` 会进入现有 desktop Extensions 视图，而不是新页面或并行平台层。
5. 任务仍旧通过 Goose 现有 `recipeId / starter prompt / skill hint` 路径启动，不因扩展推荐而改变运行时。

## Goal 10 可视回归前提

- `Computer Use` 需要当前 macOS 会话处于解锁状态
- repo 这份 `Security Goose` Electron 窗口需要是当前前台窗口
- 首选先运行 `./scripts/start-security-preview.sh`
- 如果前台不是 `Security Goose`，再执行 `./scripts/focus-security-preview-window.sh`

当前仓库内的最小可视回归建议顺序：

1. `./scripts/start-security-preview.sh`
2. `./scripts/focus-security-preview-window.sh`（如果窗口没有自动置前）
3. 用 Computer Use 检查：
   - Launcher 安全任务入口
   - Recipes 安全任务入口
   - 至少一条 recipe-backed 任务可点击
   - 至少一条 guided preview 任务可点击
   - 推荐扩展状态可见

   注意：

   - Computer Use 应优先使用当前仓库这份 Electron app 的绝对路径
   - 不要用模糊 app 名 `Electron` 做验证；当前 macOS 会话里它可能命中别的仓库或别的 Electron `default_app`

如果本机 `Computer Use` 仍受桌面无障碍或会话状态限制，仓库内的替代校验仍然是：

- `./scripts/run-security-visual-smoke.sh`
- 这条链会用 Electron + Playwright 启动当前 repo 里的 Security Goose，校验：
  - Launcher 安全任务入口可见
  - Recipes 安全任务入口可见
  - 至少一条 recipe-backed 任务可打开新窗口
  - 至少一条 guided preview 任务可打开新窗口
  - 推荐扩展状态可见
- recipe-backed 入口的真实运行时 id 仍沿用 Goose 当前 `/recipes/list` 返回的 manifest hash；desktop 只在本地把 recipe 文件 stem 映射到该 id，不引入并行 recipe loader
- `pnpm --dir ui/desktop exec vitest run src/components/LauncherView.test.tsx src/components/recipes/RecipesView.test.tsx`
- 这条替代链覆盖任务卡片、zh-CN 文案、recipe/preview 启动映射和扩展状态，但不等价于真实桌面窗口点击

## Preview backend helper smoke

如果你只想确认 preview backend helper 本身可用，而不是直接打开桌面窗口，可以执行：

```bash
./scripts/check-security-preview-backend.sh
```

这条检查会验证：

- repo 自带 preview backend helper 能选到 repo 内 `goosed`
- backend 能监听本机端口
- `https://127.0.0.1:$GOOSE_PORT/status` 在当前 secret 下可达

## macOS-only 本地打包

Apple Silicon 本地包：

```bash
source bin/activate-hermit
pnpm --dir ui/desktop run bundle:default
./scripts/check-security-macos-bundle.sh --arch arm64 --expect local-preview
```

Intel 本地包：

```bash
source bin/activate-hermit
pnpm --dir ui/desktop run bundle:intel
./scripts/check-security-macos-bundle.sh --arch x64 --expect local-preview
```

默认产物目录：

- `ui/desktop/out/Security Goose-darwin-arm64/`
- `ui/desktop/out/Security Goose-darwin-x64/`

当前 packaged app 的 Goose-first 运行时边界是：

- 打包产物仍只使用包内 `Contents/Resources/bin/goosed`
- 不接受外部 `Goose.app` 自带的 `goosed`
- packaged app 启动时会把 bundled `security-cn` 里的安全 skills / recipes 以“只补缺、不覆盖用户修改”的方式种到当前工作目录：
  - `.agents/skills/`
  - `.goose/recipes/`
- 这一步是为了把发行源目录接回 Goose 当前原生 runtime 入口，不新增 skill loader 或并行 recipe engine

当前本地 unsigned bundle 还额外固定了 3 个安装态边界：

- `bundle:default` / `bundle:intel` 会显式把 `GOOSE_DESKTOP_SIGN` 默认压成 `false`
  - 避免宿主 shell 里残留的 `APPLE_TEAM_ID` 等环境变量把本地 preview 误判成 signed build
- 本地 unsigned bundle 会把 `GOOSE_DISABLE_KEYRING=1` 写进 app 的 `LSEnvironment`
  - 目的是避免首次安装预览时触发 `Security Goose Key` 相关的钥匙串恢复/查找弹窗
- 本地 unsigned bundle 在 zip 前会执行一次 ad-hoc `codesign --force --deep --sign -`
  - 目的是把 Electron Forge 产物收口成一个 `codesign --verify --deep --strict` 可通过的本地预览 app，降低“应用程序已不能再打开”这类安装态损坏表现

签名/公证边界保持明确：

- 没有 Apple signing secrets 时：
  - 当前只保证“macOS 本地可安装预览”
  - `codesign --verify` 会通过
  - `spctl` 仍可能拒绝，因为这不是已 notarize 的正式分发包
- 有 Apple signing secrets 且 `GOOSE_DESKTOP_SIGN=true` 时：
  - 保留 cookie encryption 与系统 keyring
  - CI/release workflow 走现有 signed / notarization 链

如果你是从 zip、CI artifact 或聊天工具里解压 app，而不是直接从本机构建目录启动，第一次启动前建议执行：

```bash
xattr -dr com.apple.quarantine "/path/to/Security Goose.app"
```

这是 macOS 安装分发边界，不是 Goose runtime 边界；当前 V1a 不为了绕过它去改 Goose 核心网络或运行时架构。

## Signed release / notarization 演练边界

当前正式签名发布链仍是 Goose-first 的现有 reusable workflow：

- `bundle-desktop.yml`
- `bundle-desktop-intel.yml`
- `release.yml`
- `bundle-desktop-manual.yml`

它依赖的真实 Apple 条件是：

- `APPLE_CERTIFICATE_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_TEAM_ID`
- `APPLE_ID`
- `APPLE_ID_PASSWORD`
- 有效的 Developer ID Application 证书
- 有 notarization 权限的 Apple Developer 账号
- CI runner 能访问 Apple notarization 服务

仓库内现在有一条显式 preflight：

```bash
node scripts/check-security-apple-signing-env.mjs
```

在真的去点 GitHub signed 演练前，先检查 GitHub 侧是否具备执行条件：

```bash
node scripts/check-security-github-release-readiness.mjs
```

这条命令重点检查：

- 当前候选分支是否已推到目标 repo
- 目标 repo 是否有 `Manual Desktop Bundle` 和 macOS reusable bundle workflows
- `signing` environment 是否存在
- 当前 `gh` 身份是否具备 workflow 操作权限

如果你是要做 signed release 演练，必须用：

```bash
GOOSE_DESKTOP_SIGN=true node scripts/check-security-apple-signing-env.mjs --require-signed
```

这条命令会把“缺 secrets”和“secrets 形态明显不对”的问题在打包前暴露出来。

当前 signed/notarized 边界明确分成两层：

- 代码与 workflow 已就绪：
  - reusable workflow 会显式要求 signed preflight
  - signed bundle 期望值会额外要求 `--expect signed --require-notarized`
- 真实发布是否成功：
  - 仍取决于 Apple secrets、证书有效性和 notarization 环境
  - 如果这里失败，属于 Apple 环境 / 发布条件 blocker，不是 Security Goose 本地 preview 架构 blocker

拿到 Apple secrets 的执行者，现应直接按
[`signed-release-runbook.md`](./signed-release-runbook.md)
执行：

- 优先跑 `Manual Desktop Bundle` 的 signed 演练
- 查看 arm64 / x64 的 release evidence artifact 与 job summary
- 只有 signed 演练通过后，再推进正式 `release.yml` tag 路径

## 安装后首启 UX 检查表

对本地安装预览或 signed 候选包，至少手工确认一次：

1. Finder / Launchpad 中 app 名称显示为 `Security Goose`。
2. 启动图标和应用图标正常，没有回退成默认占位。
3. 首次进入时默认文案是中文口径，而不是英文 onboarding。
4. Settings 中 provider / model 默认接线与 `distro/security-cn/config/desktop-env.example`、`model-catalog.json` 一致。
5. Recipes 页顶部仍能看到安全任务入口区。
6. `漏洞研判`、`告警分析`、`网页调查` 仍保留 recipe-backed 链路。
7. `IOC 研判`、`报告生成`、`业务逻辑排查（WooYun-style）` 仍保留 guided preview 链路。
8. Extensions 视图里的推荐安全扩展状态没有回退。

当前仓库内自动化对这份 checklist 的覆盖边界是：

- 自动化已覆盖：
  - app metadata / bundle id / signing mode / zip 解包
  - packaged app 可启动
  - bundle 内 `goosed` 与 `security-cn` 资源存在
  - Launcher / Recipes 安全任务入口
- 仍需手工确认：
  - Finder 图标观感
  - 首次进入时的中文文案观感
  - 安装后首屏的 provider/model 体验是否符合预期

如果你要做安装态 smoke，执行：

```bash
source bin/activate-hermit
pnpm --dir ui/desktop run bundle:default
./scripts/run-security-packaged-smoke.sh
```

这条检查会验证：

- packaged app 可拉起并保持运行
- startup diagnostics 显示 backend 来自包内 `Contents/Resources/bin/goosed`
- `/status` 健康检查已经通过
- 当前工作目录下的 `.agents/skills` 与 `.goose/recipes` 已被正确补齐

当前这条 packaged smoke 额外固定了两个边界：

- 不再通过改写 `HOME` 来做隔离，避免触发 macOS 的 `Security Goose Key` 钥匙串恢复弹窗
- unsigned 本地 bundle 默认不再要求 Electron 把本地 cookie/storage key 写入 macOS Keychain；signed build 仍保留该能力
- bundle 前会先停掉当前仓库 `out/.../*.app` 下仍在运行的 packaged 实例，避免重打包后留下“应用程序已不能再打开”的失效 app 进程

## 不做的事

- 不新建并行 runtime
- 不新建并行 memory system
- 不新建并行 tool scheduler
- 不做 gateway
- 不做 LiteLLM
- 不做 AGS
- 不做在线 marketplace
- 不做企业后台
