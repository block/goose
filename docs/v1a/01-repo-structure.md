# V1a 新 Goose 主线仓库目录结构

## 设计原则

新仓库必须**沿用 Goose 原始结构**，只在最少必要位置增加产品自定义目录。

V1a 额外约束：

- `macOS-only`
- 仅本机预览
- 首期不默认引入 `services/gateway/`
- 首期不默认引入自定义 plugin bundle

结构目标：

- 上游 Goose 易于同步
- 定制点可定位、可审查、可回滚
- 不把安全产品逻辑散落到 Goose core 各处

## 建议仓库名

- GitHub 仓库：`security-goose`
- 产品代号：`Security Goose`
- 分发名称：后续替换为正式品牌名

## 推荐目录树

```text
security-goose/
├── bin/                              # 沿用 upstream hermit / helper scripts
├── crates/                           # Goose Rust core，尽量少改
│   ├── goose/                        # Provider / agent / config / prompts / extensions
│   ├── goose-server/                 # goosed
│   ├── goose-mcp/                    # built-in MCP servers
│   └── ...                           # upstream crates
├── ui/
│   └── desktop/                      # Goose Electron + React 桌面端
├── distro/
│   └── security-cn/                  # 本产品的所有发行版定制尽量集中到这里
│       ├── branding/
│       │   ├── app-icon.icns
│       │   ├── app-icon.png
│       │   ├── splash.png
│       │   └── product-metadata.json
│       ├── config/
│       │   ├── init-config.yaml.example
│       │   ├── desktop-env.example
│       │   ├── provider-defaults.yaml
│       │   ├── model-catalog.json
│       │   └── feature-flags.json
│       ├── locales/
│       │   ├── zh-CN.json
│       │   └── en-US.json
│       ├── prompts/
│       │   ├── system-zh.md
│       │   ├── system-en.md
│       │   └── security-role-defaults.md
│       ├── skills/
│       │   ├── vuln-triage/
│       │   │   ├── SKILL.md
│       │   │   └── templates/
│       │   ├── alert-triage/
│       │   ├── ioc-analysis/
│       │   ├── asset-risk-summary/
│       │   └── report-writing/
│       ├── recipes/
│       │   ├── security-vuln-triage.yaml
│       │   ├── alert-investigation.yaml
│       │   └── web-investigation.yaml
│       ├── extensions/
│       │   ├── bundled-extensions.security.json
│       │   ├── aiseesec-mcp/
│       │   ├── local-security-gateway-mcp/
│       │   ├── threat-intel-mcp/
│       │   └── browser-assist-mcp/
│       └── docs/
│           ├── operator-guide.md
│           └── capability-catalog.md
├── scripts/
│   ├── sync-upstream.sh
│   ├── build-distro.sh
│   ├── validate-skills.mjs
│   ├── validate-locales.mjs
│   ├── validate-recipes.mjs
│   ├── validate-extensions.mjs
│   └── export-model-catalog.mjs
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   ├── release-desktop.yml
│   │   ├── docs.yml
│   │   └── upstream-sync-check.yml
│   ├── ISSUE_TEMPLATE/
│   └── PULL_REQUEST_TEMPLATE.md
├── docs/
│   ├── PRODUCT.md
│   ├── ARCHITECTURE.md
│   ├── SKILLS.md
│   ├── MCP.md
│   ├── TESTING.md
│   ├── RELEASE.md
│   ├── LOCALIZATION.md
│   └── DEVELOPMENT.md
└── README.md
```

## 为什么这样分

## `crates/`

这里是 Goose 的核心能力层，必须尽量保持上游结构：

- session 管理
- agent 执行
- provider
- extension loading
- memory
- prompts

原则：

- 非必要不改
- 如需改，优先提交上游或保持补丁极小

## `ui/desktop/`

这里是 Goose Desktop 主体：

- Electron 壳
- React UI
- 桌面设置页
- 会话界面

原则：

- 只做品牌、文案、入口、少量产品化 UI 调整
- 不重写整套桌面 UI

## `distro/security-cn/`

这是**新主线最关键的产品定制目录**。

所有安全产品差异，优先放这里：

- 品牌
- 中文/英文文案
- 默认 prompt
- 安全 skills
- recipes
- bundled MCP
- 模型目录

补充说明：

- `init-config.yaml.example` 是 Goose core 首次配置的源文件
- `desktop-env.example` 是桌面默认 locale / predefined models 的源文件
- `distro/security-cn/**` 当前是发行素材目录，不是 Goose 自动扫描目录

这样做的价值：

- 迁移、审查、打包都简单
- 上游升级时不容易冲掉核心定制

## `scripts/`

只放与产品发行相关的脚本：

- 上游同步
- distro 打包
- skill 校验
- i18n 校验
- model catalog 导出

## 新仓库里的“可改 / 慎改 / 禁改”

## 可改

- `distro/security-cn/**`
- `docs/**`
- `scripts/**`
- `ui/desktop/` 中少量 branding 与产品入口

## 慎改

- `ui/desktop/src/components/**`
- `crates/goose/src/prompts/**`
- `ui/desktop/src/built-in-extensions.json`
- `ui/desktop/src/components/settings/extensions/bundled-extensions.json`

## 禁止作为 V1 默认动作

- 大范围重构 `crates/goose/**`
- 自建 parallel session store
- 自建 parallel memory store
- 重写 `goosed`
- 重写桌面壳
- 在 V1a 首期引入自定义 gateway runtime

## 初始化建议

1. fork `aaif-goose/goose`
2. 新仓库默认分支设为 `main`
3. 保留 `upstream` remote 指向上游 Goose
4. 第一条实现分支建议：
   - `codex/goose-v1a-bootstrap`
5. 第一个落地目录优先创建：
   - `distro/security-cn/`
   - `docs/`

## V1 目录验收标准

- 新仓库结构仍能看出是 Goose，不是重新造的一套项目
- 产品定制集中在 `distro/security-cn/`
- 首期不依赖额外业务后端也能跑通
- 文档、脚本、CI、打包路径一开始就有明确位置
