# security-cn distro

`distro/security-cn/` 是 V1a 的发行源目录，不是 Goose 当前自动扫描目录。

## 目录职责

- `branding/`
  Goal 3 的品牌名、图标、打包元数据源文件
- `config/`
  provider 默认值、模型目录、桌面环境变量示例
- `locales/`
  中文/英文产品文案源文件，后续合并到 `ui/desktop/src/i18n/messages/`
- `prompts/`
  安全角色与系统提示词源文件，后续接到 Goose prompt 原生入口
- `skills/`
  安全 skill 源目录，后续发布到项目 `.agents/skills/`
- `recipes/`
  安全 recipe 源目录，后续发布到 `.goose/recipes/` 或 `GOOSE_RECIPE_PATH`
- `extensions/`
  MCP wrapper 与 bundled extension catalog 源目录
- `docs/`
  发行使用说明与能力目录

## Goose 原生入口映射

| distro 源目录 | Goose 当前可用入口 |
| --- | --- |
| `config/init-config.yaml.example` | 工作区根目录 `init-config.yaml` |
| `config/desktop-env.example` | Electron 进程环境变量 |
| `config/model-catalog.json` | `GOOSE_PREDEFINED_MODELS` 的源文件 |
| `locales/*.json` | `ui/desktop/src/i18n/messages/*.json` |
| `prompts/*.md` | `crates/goose/src/prompts/*.md` 或 `~/.config/goose/prompts/*.md` |
| `skills/*/` | 项目 `.agents/skills/` |
| `recipes/*.yaml.example` | 项目 `.goose/recipes/` |
| `extensions/bundled-extensions.security.json.example` | `ui/desktop/src/components/settings/extensions/bundled-extensions.json` |

## 当前阶段约束

- 不在这里新建并行 runtime
- 不把这些目录直接接成自定义 loader
- 先保留为发行素材与落位约定

## Goal 5 本机预览落位

- `distro/security-cn/skills/*/SKILL.md` 继续作为发行源文件
- `scripts/sync-security-runtime-assets.mjs` 会把整个 skill 目录镜像到项目 `.agents/skills/`
- `distro/security-cn/recipes/*.yaml.example` 继续作为发行源文件
- `scripts/sync-security-runtime-assets.mjs` 会把它们镜像到项目 `.goose/recipes/*.yaml`
- security MCP catalog 只同步到 `ui/desktop/src/components/settings/extensions/bundled-extensions.json`
  原因：这是桌面当前真正会同步进 config 的入口；`ui/desktop/src/built-in-extensions.json` 仍只保留 upstream builtin 展示数据
- `wooyun-legacy` 作为 Goose-native 包装 skill 内置；上游 `CC BY-NC-SA 4.0` 参考包不随 fork 分发，只能通过本地安装脚本装到 runtime

## Goal 6 桌面入口落位

- 快速入口：`ui/desktop/src/components/LauncherView.tsx`
  - 面向“可见、可点、可快速启动”的安全任务入口
  - 通过 `window.electron.createChatWindow({ query, recipeId })` 复用 Goose 现有 launcher / pair 机制
- 模板入口：`ui/desktop/src/components/recipes/RecipesView.tsx`
  - 在 Recipes 页顶部提供精选安全任务卡片
  - recipe 已存在时走 Goose 原生 recipe runtime
  - 无现成 recipe 时退化为 skill 引导预览对话

这一层只做任务语义到 `recipeId / starter prompt / skill hint` 的薄映射，不新增并行任务编排层。

## Goal 7 校验与打包落位

- 仓库内最小自动校验统一走 `scripts/check-security-v1a.sh`
  - 复用 hermit、desktop vitest、`tsc --noEmit`、`lint:check`、`scripts/validate-security-distro.mjs`
  - 不改 upstream 通用 CI 主链，只补一个 `security-goose-v1a-checks.yml` 的独立 workflow
- Goal 6 安全入口 smoke
  - 组件测试：`ui/desktop/src/components/LauncherView.test.tsx`
  - 组件测试：`ui/desktop/src/components/recipes/RecipesView.test.tsx`
  - 手工步骤：见 `docs/operator-guide.md`
- macOS-only 本机打包继续走 desktop 现有脚本
  - `pnpm --dir ui/desktop run bundle:default`
  - `pnpm --dir ui/desktop run bundle:intel`

## Goal 8 扩展落位

- `browser-assist-mcp` 与 `threat-intel-mcp` 继续以 `distro/security-cn/extensions/**/server.mjs` 作为发行源文件
- desktop bundled extension 同步时，会把这两条 security stdio 脚本参数解析成绝对路径后再写入 Goose config
  原因：Goose extension 子进程 `cwd` 跟随 session 工作目录，不能依赖相对路径
- `scripts/smoke-security-extensions.mjs` 使用真实 MCP 握手对这两条链路做仓库内 smoke
- `aiseesec-mcp` 与 `local-security-gateway-mcp` 仍只保留 disabled stub 与 blocker 说明

## Goal 9 任务与扩展联动

- `ui/desktop/src/security/taskCatalog.ts` 继续作为任务入口的单一映射层
- Goal 9 只在这个映射层上补 `recommendedExtensionIds`
  - 不新增并行任务编排层
  - 不改变 Goose recipe / skill / MCP runtime
- desktop 当前两处真实承载位：
  - `ui/desktop/src/components/LauncherView.tsx`
  - `ui/desktop/src/components/recipes/RecipesView.tsx`
- 呈现内容严格分三类：
  - `local preview`
  - `disabled stub`
  - `blocked by external dependency`
