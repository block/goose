# Goose V1a Bootstrap Audit

## 目标

这份审计文档只回答当前阶段两个问题：

1. Goose fork 现在有哪些原生入口可以直接复用
2. `docs/v1a` 与 `distro/security-cn` 应该怎样落位，才不会和 Goose 现状冲突

## 已确认的 Goose 原生入口

| 需求 | Goose 当前入口 | 代码位置 | 当前结论 |
| --- | --- | --- | --- |
| 首次默认配置 | 工作区根目录 `init-config.yaml` | `crates/goose/src/config/base.rs` | 原生支持 |
| 桌面默认 provider / model | Electron 进程环境变量 | `ui/desktop/src/main.ts` | 原生支持 |
| 桌面默认 locale | `GOOSE_LOCALE` 环境变量 | `ui/desktop/src/main.ts`, `ui/desktop/src/i18n/index.ts` | 原生支持 |
| 预置模型列表 | `GOOSE_PREDEFINED_MODELS` 环境变量 | `crates/goose/src/model.rs`, `ui/desktop/src/components/settings/models/predefinedModelsUtils.ts` | 原生支持，但必须是 JSON 字符串 |
| Prompt 覆写 | `~/.config/goose/prompts/*.md` | `crates/goose/src/prompt_template.rs` | 原生支持 |
| Skill 发现 | `.agents/skills`, `.goose/skills`, `~/.config/goose/skills` 等 | `crates/goose/src/skills/mod.rs` | 原生支持 |
| Recipe 发现 | 当前目录、`GOOSE_RECIPE_PATH`、`.goose/recipes`、`~/.config/goose/recipes` | `crates/goose/src/recipe/local_recipes.rs` | 原生支持 |
| Bundled extensions 目录 | 桌面内置 JSON 目录 | `ui/desktop/src/built-in-extensions.json`, `ui/desktop/src/components/settings/extensions/bundled-extensions.json` | 原生支持 |

## 与 spec 的冲突

### 1. `docs/v1a/README.md` 仍然指向旧 CSO 仓库路径

问题：

- 文档已经迁入当前 Goose fork
- 但目录地图和示例链接仍然跳到旧的 `CSO` 绝对路径

最小修正：

- 全部改为当前仓库内的相对链接

### 2. `GOOSE_PREDEFINED_MODELS` 不能写成逗号分隔字符串

问题：

- Goose 当前实现按 JSON 数组解析 `GOOSE_PREDEFINED_MODELS`
- 逗号分隔字符串会在桌面端和 core 端同时解析失败

最小修正：

- `model-catalog.json` 作为源文件保存模型数组
- 桌面打包或本机预览时，把这个 JSON 文件内容注入 `GOOSE_PREDEFINED_MODELS`

### 3. `GOOSE_DEFAULT_LOCALE` 不是桌面默认语言入口

问题：

- 桌面端当前读取的是 `GOOSE_LOCALE`
- `init-config.yaml` 里的 `GOOSE_DEFAULT_LOCALE` 不会自动进入 Electron 进程环境

最小修正：

- 本机预览和打包时使用 `GOOSE_LOCALE=zh-CN`
- `init-config.yaml` 只承担 goosed / Goose core 的首次默认配置

### 4. `distro/security-cn/{locales,prompts,skills,recipes,extensions}` 不会被 Goose 自动扫描

问题：

- 这些目录是很好的发行素材落点
- 但 Goose 当前不会直接从这里发现 locale、prompt、skill、recipe 或 extension catalog

最小修正：

- 继续保留 `distro/security-cn/**` 作为发行源目录
- Goal 3 以后分别接到 Goose 原生入口：
  - locales -> `ui/desktop/src/i18n/messages/*.json`
  - prompts -> `crates/goose/src/prompts/*.md` 或 `~/.config/goose/prompts/*.md`
  - skills -> 项目 `.agents/skills/`
  - recipes -> `.goose/recipes/` 或 `GOOSE_RECIPE_PATH`
  - bundled extensions -> 桌面 `bundled-extensions.json`

### 5. 当前 Goose recipe schema 没有顶层 `skills` 字段

问题：

- 当前公开 schema 支持 `instructions`、`extensions`、`settings`、`activities` 等
- 本阶段不应把未证实的 `skills` 字段直接作为运行时格式

最小修正：

- 先把安全 recipe 作为发行素材示例保存
- 运行时 recipe 用 `instructions + extensions + settings`
- Goal 5 再决定 skill 与 recipe 的最终接线方式

## 当前阶段的落位结论

### `docs/v1a/`

合理，继续作为：

- V1a spec
- 迁移与阶段验收依据
- 发行素材示例文档

### `distro/security-cn/`

合理，但要明确它是：

- 发行源目录
- 不是 Goose 当前自动扫描目录

因此本阶段只做：

- 骨架
- 示例文件
- 入口映射说明

不做：

- 自建运行时
- 自建 locale loader
- 自建 recipe engine

## 当前启动链审计补充

### repo 自带 `goosed` 现在如何进入 desktop

- 官方本机预览入口是 `./scripts/start-security-preview.sh`
- 它最终调用 `pnpm --dir ui/desktop run start-gui -- --dir "$ROOT_DIR"`
- 但它在进入 Electron 之前，会先用 `scripts/launch-security-preview-backend.sh` 从 shell 拉起 repo 自带 backend，再通过 `GOOSE_EXTERNAL_BACKEND=1` 走 Goose 现有 external-backend 链路
- `start-gui` 先执行 `ui/desktop/scripts/ensure-goosed-dev.js`
- `ensure-goosed-dev.js` 只接受仓库内 3 个路径：
  - `target/release/goosed`
  - `target/debug/goosed`
  - `ui/desktop/src/bin/goosed`
- desktop 开发态现在也按这组优先级查找 `goosed`
- macOS 本机预览现在默认优先 staging `target/release/goosed`
- 当前观察到两个不同层次的 blocker：
  - repo 自编 `target/debug/goosed` 可能卡在 `_dyld_start`，表现为“进程存在但不监听端口”
  - `ui/desktop/src/bin/goosed` 在当前 macOS 会话里也可能间歇性卡在 `_dyld_start`，即使它和 `target/release/goosed` 字节完全一致
- 这意味着 `src/bin/goosed` 现在只能作为 fallback / packaging staging 副本，不能当作当前 preview 链的稳定首选
- 这个 blocker 属于本地 debug 二进制可执行性问题，不是 Goose desktop 页面逻辑失败

这意味着当前预览链已经回到 Goose 原生 desktop shell + repo 自编 `goosed`，而不是依赖外部安装版 `Goose.app`。

同时，官方预览包装脚本会在启动前只清理当前仓库这份 preview 的旧 Electron 与 `goosed` 进程，避免 `requestSingleInstanceLock()` 把新启动错误复用到失效旧窗口。

另外，当前已确认一个与可视回归相关的环境特征：

- repo Electron 窗口默认可能不会自动抢到前台
- 这会让 `Computer Use` 很难在第一时间抓到 `Security Goose` 的活动窗口
- 当前最小修正是把前台激活留在官方预览包装脚本里，而不是改 Goose runtime 或另造 UI 测试层
- 如果 `Computer Use` 后续仍因桌面会话或工具状态机导致点击不稳定，仓库内应优先使用 Electron + Playwright 的 `./scripts/run-security-visual-smoke.sh` 做等价交互 smoke，而不是继续新造并行 UI 测试平台
- 另一个已确认的 Goose 现状冲突是：`/recipes/list` 暴露的 `manifest.id` 是路径 hash，不是 `security-vuln-triage` 这样的文件 stem；当前最小修正放在 desktop 侧，用 `file_path` stem 映射回运行时 id，不改 recipe runtime 协议

### 当前剩余 blocker 是什么

当前剩余问题不是 desktop 代码路径错误，而是冷缓存环境下的 Rust crates 下载速度与镜像可用性：

- 如果本机已经能完成 `cargo build -p goose-server --bin goosed`，desktop 预览可直接使用 repo 自带二进制
- 如果首次构建卡在 crates 拉取，可临时设置 `GOOSE_CARGO_REGISTRY_MIRROR=rsproxy-cn`
- 这是下载源 fallback，不是新的平台层，也不是对 Goose runtime 的改造

另外，direct `pnpm --dir ui/desktop run start-gui` 需要单独说明边界：

- 它仍然不是本仓库的官方 preview 入口
- 但当前已进入“开发态支持”范围，可用于主进程排障和 startup diagnostics 复现
- 本轮已确认一个直接根因：
  - `pnpm run ... -- --dir "$ROOT_DIR"` 进入 Node 包装脚本后，Node 会吞掉参数分隔用的 `--`
  - 如果包装脚本不把这个分隔符补回 `electron-forge start -- --dir "$ROOT_DIR"`，`--dir` 就会误落到 Forge CLI 层，Electron 主进程拿不到 repo 工作目录
  - 修正后，direct 启动可稳定进入 repo 主界面，并在隔离的 `GOOSE_USER_DATA_DIR` 下产生新的 `main.log` 和 startup diagnostics
- 如要直接使用，应满足两个前提：
  - 当前仓库已经先有 `target/release/goosed`
  - 当前没有旧的 repo preview Electron / `goosed` 进程残留
- 如果还要复用 wrapper 一样的 backend 选择、secret 注入和用户数据隔离，仍应先用 `scripts/launch-security-preview-backend.sh` 导出环境变量
- 否则你看到的窗口、日志和 startup diagnostics 可能混入旧进程状态，给“是 backend 没起来还是复用了旧窗口”制造歧义

另外，前台聚焦链路也有一个已确认的误判来源：

- 直接对通用 `Electron.app` 做 activate，可能会拉起别的仓库或别的默认 Electron `default_app`
- 当前最小修正已经改为只根据 repo preview 进程 pid 提升对应窗口到前台
- 因此 `Computer Use` 做最终验证时，也应优先使用当前仓库这份 Electron app 的绝对路径，而不是模糊 app 名 `Electron`

### packaged 安装态的最小修正

当前已确认一个 Goose 现状与安全发行目录的天然落差：

- packaged app 可以从 `Contents/Resources/security-cn` 读取 branding 与默认配置
- 但 Goose runtime 不会自动从这个 bundled 发行目录扫描 skills / recipes

因此本阶段的最小修正是：

- 只在 packaged 启动链里做一个幂等 bootstrap
- 把 bundled `security-cn/skills/**` 补到当前工作目录 `.agents/skills/`
- 把 bundled `security-cn/recipes/*.yaml.example` 补到当前工作目录 `.goose/recipes/*.yaml`
- 只补缺，不覆盖用户已经改过的运行时文件

这仍然遵循 Goose-first：

- runtime 继续只认 Goose 当前原生技能/配方目录
- 不新增 skill loader
- 不新增 recipe engine
- 不引入并行安装器或后台管理层

### macOS 本地安装预览边界补充

这一轮又确认了两个安装态层面的现实问题：

1. 仅靠 `APPLE_TEAM_ID` 是否存在去推断“应该启用 signed build 行为”，对本地开发机不够稳
   - 因为开发机 shell 里可能残留 Apple 相关环境变量
   - 这会让一个本来只想做本机 preview 的 `bundle:default` 意外启用 Electron cookie encryption
   - 最直接的表现就是首次启动出现 `Security Goose Key` 相关钥匙串弹窗

2. Electron Forge 产出的本地 app，在 unsigned 路径下不能只看“能否跑起来”
   - 当前需要再做一次本地 ad-hoc re-sign
   - 否则可能出现 `codesign --verify` 不干净，进一步演化成“应用程序已不能再打开”这类安装态问题

当前最小修正已经收口为：

- 用显式环境变量 `GOOSE_DESKTOP_SIGN` 区分：
  - `local-preview`
  - `signed`
- 本地 `bundle:default / bundle:intel` 默认固定 `GOOSE_DESKTOP_SIGN=false`
- `local-preview` bundle：
  - 关闭 Electron cookie encryption
  - 在 `Info.plist -> LSEnvironment` 中写入 `GOOSE_DISABLE_KEYRING=1`
  - zip 前做一次 ad-hoc `codesign --force --deep --sign -`
- CI reusable bundle workflow：
  - 显式把 `GOOSE_DESKTOP_SIGN` 绑定到 `inputs.signing`
  - 并在上传 artifact 前跑 `scripts/check-security-macos-bundle.sh`
  - signed 模式下还会先跑 `scripts/check-security-apple-signing-env.mjs --require-signed`
  - signed bundle 还会额外要求 notarization 证据检查

这仍然符合 Goose-first：

- 没有改 Goose backend / desktop 协议
- 没有新增后台管理器
- 没有新增并行安装层
- 只是把 macOS 打包态的显式边界补回到现有 bundle 链路

### signed release 演练边界补充

这一轮再往前收口一层后，当前链路可以明确分成两类 blocker：

1. 代码 / workflow blocker
   - 例如没有显式区分 `local-preview` 与 `signed`
   - 或者 signed 路径没有前置 secrets 审计
   - 或者 bundle 产物没有最基本的 `codesign / stapler / spctl` 证据输出

2. Apple 环境 blocker
   - 例如 GitHub `signing` environment 下缺少证书与 Apple ID secrets
   - 证书本身无效、过期、权限不对
   - notarization 服务访问失败

当前仓库已经把第 1 类问题收口到了显式脚本和 workflow：

- `scripts/check-security-apple-signing-env.mjs`
- `scripts/check-security-macos-bundle.sh --expect signed --require-notarized`

因此后续如果 release tag 流水线失败，先看：

- 是 preflight 报缺 secrets
- 还是 bundle 验证缺 notarization 证据

这两类都会直接指向“Apple 发布条件没满足”，而不是再把问题混回 Goose runtime 本身。

本轮再补上 release evidence 汇总后，当前仓库已经进入这样一个边界：

- 没有 Apple secrets 的执行者：
  - 仍然只能停留在 `local-preview` / packaged smoke / signing preflight 失败演练
- 拿到 Apple secrets 的执行者：
  - 可以直接走 `Manual Desktop Bundle` 的 signed 演练路径
  - 可以从 arm64 / x64 artifact 和 job summary 读取固定格式的 preflight / bundle / spctl / stapler 证据
  - 如果 signed 演练通过，再触发正式 `release.yml` tag 路径

因此当前剩余 blocker 已经进一步缩小到：

- Apple Developer 证书与账号权限
- GitHub `signing` environment 的真实 secrets
- runner 到 Apple notarization 服务的连通性

另外，到了真实 signed rehearsal 阶段，还需要再满足一个仓库态前提：

- 当前候选分支必须已经推到目标 GitHub repo
- 目标 repo 必须真的暴露 `Manual Desktop Bundle` 与 macOS reusable bundle workflows

否则你即使本地代码已收口，也无法用 GitHub Actions 对“当前这份候选代码”做正式 signed 演练。

### 本机 Rust lint 口径补充

当前仓库根目录 `rust-toolchain.toml` 固定在 `1.92`，但在当前 macOS `aarch64` 会话里，
这个 toolchain 不能直接提供 `cargo-clippy`：

- `cargo clippy --workspace --all-targets --exclude v8 -- -D warnings`
  - 失败原因是 toolchain 组件不可用，不是源码编译失败

当前与 upstream `ci.yml` 的 `rust-lint` job 对齐的本机最小替代命令是：

- `cargo +stable clippy --workspace --all-targets --exclude v8 -- -D warnings`

原因：

- upstream CI 本身就不会直接依赖 hermit 管理的 rustup 来跑 clippy
- `ci.yml` 里的 `rust-lint` job 已显式执行 `hermit uninstall rustup`
- 因此本机使用系统 `stable` toolchain 跑 clippy，和 CI 的实际 lint 口径一致

## 本阶段建议的最小工作方式

1. `init-config.yaml.example` 只保留 Goose core 真正能吃到的首次默认配置
2. 用 `desktop-env.example` 承担桌面默认 locale / predefined models
3. 用 `model-catalog.json` 作为 `GOOSE_PREDEFINED_MODELS` 的源文件
4. 用 `distro/security-cn/README.md` 说明每个目录未来如何接到 Goose 原生入口
5. 用一个轻量校验脚本保证骨架和示例文件不再偏离 Goose 当前实现
