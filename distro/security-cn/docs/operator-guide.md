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

   如果你本地固定使用腾讯云 Token Plan，可以直接把本机 key 写到工作区根目录 `init-config.yaml` 的后端配置里。
   这个文件应只作为本机 server config 使用，不要提交到 git：

   ```yaml
   GOOSE_PROVIDER: openai
   OPENAI_BASE_URL: https://tokenhub.tencentmaas.com/plan/v3
   OPENAI_API_KEY: sk-...
   GOOSE_MODEL: deepseek-v4-flash
   GOOSE_TELEMETRY_ENABLED: true
   GOOSE_POSTHOG_API_HOST: https://us.i.posthog.com
   GOOSE_POSTHOG_PROJECT_API_KEY: phc_yS3ZTSB2WBmKf6aiBHstbfV4Nc2cxc7KxVavBxNjBBSn
   ```

   说明：

   - `OPENAI_BASE_URL` 作为普通 Goose 配置项，会通过 `init-config.yaml` 注入
   - `OPENAI_API_KEY` 对 Goose 原生 `openai` provider 属于 secret，preview/backend 启动链会把本地 `init-config.yaml` 里的该值提升成后端进程环境变量；renderer 仍不会直接接触真实 key
   - `GOOSE_TELEMETRY_ENABLED: true` 会让 repo preview、packaged preview 和当前工作目录下的本机会话默认开始向当前 PostHog 项目发送匿名使用统计；如果你不想上报，可改回 `false` 或单次导出 `GOOSE_TELEMETRY_OFF=1`
   - `GOOSE_POSTHOG_API_HOST` 与 `GOOSE_POSTHOG_PROJECT_API_KEY` 现在会由 Goose 原生 telemetry 后端读取；收到 默认已切到当前 PostHog US Cloud 项目 `471737`，不再写死发往上游 Goose 官方项目

   当前边界：

   - official preview wrapper 会自动把当前仓库根目录 `init-config.yaml` 注入到 repo 自带 `goosed`
   - direct `pnpm --dir ui/desktop run start-gui -- --dir "$PWD"` 也会把当前工作目录下的 `init-config.yaml` 传给 desktop 拉起的本地 `goosed`
   - 前端主路径现在只强调模型切换；provider host / API key 不会下发到 renderer
   - onboarding 默认会直接引导到内置 Token Plan 后端；自由 provider / key 配置只保留为高级路径
   - 当前默认模型固定为 `deepseek-v4-flash`；`Auto` 仍保留在模型列表中，但不再作为默认值，因为当前 Token Plan `auto` 路由会偶发只返回 `thinking` 而没有最终文本
   - 当前桌面端会默认隐藏模型价格/费用显示：腾讯云 Token Plan 的官方价格虽然存在，但 Goose 现有 canonical pricing 链路不能正确表达这条后端的人民币/阶梯计价，继续显示只会误导

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
   - 启动后会尝试把当前仓库这份 `收到` Electron 窗口自动切到前台，方便 Computer Use 和人工可视回归
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

## Skills 管理面 V2

- `Settings -> Skills` 现在继续沿 Goose 原生 runtime 做当前项目管理，不新增并行 skill loader。
- 页面顶部现在直接说明“这里会显示哪些技能”，不再展示“当前项目技能数量 / 异常数量”这类实现者视角统计。
- 每个技能条目都可打开只读详情：
  - 技能名称
  - 描述
  - skill 文件夹路径
  - 去掉 frontmatter 后的 `SKILL.md` 正文
  - 当前项目运行时异常诊断说明（仅在缺失 / 覆盖 / 无效时显示）
- Finder 边界：
  - 当前项目 skill 只提供“在 Finder 中打开 skill 文件夹”
  - bundled security skill 不再额外暴露 bundled 源目录入口
  - builtin Goose skill 只显示说明，不会伪装成当前项目目录可管理
- 当前项目操作反馈：
  - 打开 skill 文件夹成功：显示已在 Finder 打开当前 skill 文件夹
  - 打开 skill 文件夹失败：显示失败原因，便于区分路径不存在、权限问题或系统打开失败
  - 导入 skill 成功：显示已安装到当前项目 runtime
  - 导入 skill 失败：显示失败原因；格式无效仍会继续弹出原生错误框
  - 删除本地 skill 成功：显示已从当前项目 runtime 删除
  - 删除本地 skill 失败：显示失败原因
  - 恢复 bundled 成功：显示已恢复到当前项目 runtime
  - 恢复 bundled 失败：显示失败原因
- invalid skill 修复建议：
  - 详情页会根据 invalid code 给出最小修复建议
  - 仍然不自动修复，也不改变 Goose 原生 discoverability 逻辑
- slash-command 与 Skills 页边界：
  - `Skills` 页显示的是当前产品允许浏览和管理的技能范围
  - `/` 菜单显示的是当前 Goose runtime 这一轮真正发现到的 skill 命令
  - 新导入 skill 可能先出现在 `Skills` 页，再在重新打开会话后出现在 `/` 菜单
  - 这是 Goose 原生 discoverability 边界，不是新的 skill loader
- skill 直启会话：
  - 只有当前 `/` 菜单已发现到的 skill，才会在 Skills 卡片和详情页里显示可用启动路径
  - 无参数 skill：直接复用 Goose 原生 slash-command，新建会话时发送 `/skill-name`
  - 有参数提示的 skill：先弹出一个最小输入框，收集必填参数，再复用 Goose 原生 slash-command 新建会话
  - 当前实现不会新建并行 skill 执行层，也不会伪装成“已观测到 skill 一定被运行时加载”
  - 如果某个 skill 当前还没被 `/` 菜单发现，详情页会直接说明原因，而不是静默失败

## Apps 内置安全工具

- `Apps` 页当前继续走 Goose 原生 `apps` runtime，不新建并行安全工具平台。
- 本轮把原来的默认内置示例 app 收口为一组内置安全小工具：
  - `ioc-toolbox`
  - `encode-hash-lab`
  - `secret-credential-scanner`
  - `jwt-inspector`
- 这 4 个工具都是本机 HTML/JavaScript 小工具：
  - 默认离线可用
  - 不依赖 gateway
  - 不依赖在线 marketplace
  - 不引入新的后台管理器
- 当前定位：
  - `ioc-toolbox`：面向告警、威胁情报和应急中的混合 IOC 粘贴、离线归一化、去重、按类复制和结构化导出
  - `encode-hash-lab`：面向取证、payload 分析、JSON 整理、JWT 检查和常见编码/哈希操作链处理
  - `secret-credential-scanner`：面向日志、配置、代码片段和工单中的敏感信息暴露排查
  - `jwt-inspector`：面向认证、API、IAM 和 token claims 快速检查
- `ioc-toolbox` 当前工作流边界：
  - 支持直接粘贴大段混合文本、JSON 片段、告警备注、域名/IP/URL/Hash/CVE 混合列表
  - 结果区会展示原始片段、规范化条目、唯一指标、重复收敛、识别类别和未识别内容数量
  - 默认支持三种二次利用：
    - 复制分组 JSON
    - 复制规范化列表
    - 按单一类别复制结果
  - 当同时存在可识别指标与未识别内容时，未识别内容会默认收起，避免干扰主研判结果
  - 当前仍是离线规则提取工具，不替代需要联网 enrichment 的威胁情报查询
- `encode-hash-lab` 当前工作流边界：
  - V2 已升级为本地离线多步 operation pipeline，不再只是单步按钮式编码工具
  - 当前输入与输出主路径：
    - 原始输入
    - 当前操作链
    - 逐步结果
    - 最终输出
  - 当前高频能力矩阵：
    - 已支持：
      - Base64 / Base64URL / Hex / URL / HTML Entity / Unicode Escape 编码解码
      - MD5 / SHA1 / SHA256 / SHA512
      - JSON Pretty / JSON Minify
      - 标准化换行 / 去首尾空白
      - JWT 本地解析
    - 部分对标 CyberChef：
      - 已有多步串联、顺序调整、删除步骤、逐步结果查看
      - 但还没有拖拽式步骤编辑、复杂参数型 operation、几百项全量操作库
    - 当前未支持：
      - Base32 / Ascii85 / Quoted-Printable / Brotli / CRC / HMAC 等更宽操作面
      - 更复杂的二进制视图、提取器和高级取证转换
  - 当前二次利用方式：
    - 复制最终结果
    - 复制单步结果
    - 直接把多步链路结果粘贴回工单、研判记录或后续任务模板
  - 当前结论：
    - 这版已经能覆盖中文安全从业者高频编码分析路径
    - 但仍不是 CyberChef 的全量替代品，后续要继续补更广 operation catalog 才能进一步接近
- `secret-credential-scanner` 当前工作流边界：
  - 中文优先，面向 SOC、应急、云安全、开发安全和日常工单排查
  - 适合粘贴：
    - 日志
    - 配置文件
    - 代码片段
    - 环境变量
    - HTTP 片段
    - 告警备注 / 工单内容
  - 当前会输出：
    - 原始命中数
    - 唯一敏感项数
    - 识别类别数
    - 高风险数量
    - 分类结果
    - 结构化明细
  - 当前重点覆盖：
    - Bearer token
    - JWT
    - Basic Auth
    - Cookie 片段
    - 腾讯云 / 阿里云 / 华为云凭据
    - 企微 / 钉钉 webhook 或 secret
    - 私钥 / PEM
    - 数据库连接串
    - 通用 token 化 URL
    - 常见 API key / secret-like 赋值片段
  - 当前二次利用方式：
    - 复制结构化 JSON
    - 复制规范化列表
    - 按单个类别复制结果
  - 当前替换 `header-diff-lab` 的原因：
    - 高频度更高
    - 适用岗位更广
    - 更符合“默认内置离线安全小工具”的通用定位
  - 当前边界：
    - 仍是离线规则识别工具，不验证凭据是否真的可用
    - 不替代正式的 secrets 管理、云平台审计或代码仓库扫描平台
- `jwt-inspector` 当前工作流边界：
  - 中文优先，面向认证、API、IAM 和网关 token 结构检查
  - 当前界面已收口为 Goose 风格扁平布局，不再使用 card / panel 式结果卡片
  - 当前会输出：
    - Header JSON
    - Payload JSON
    - 签名状态与签名段预览
    - `iat / nbf / exp` 时间字段
    - 风险提示
  - 当前风险提示覆盖：
    - `alg=none`
    - 签名段为空
    - `exp` 已过期
    - 缺少 `iss / sub / aud / exp`
    - `nbf` 晚于当前时间
    - `typ` 与 JWT 预期不一致
  - 当前二次利用方式：
    - 复制结构化结果 JSON
    - 单独复制签名段
    - 单独复制 Header / Payload JSON
  - 当前展示特点：
    - 长 token、长 claim 和较长 base64url 字段会优先换行并保留滚动，不再依赖大卡片堆叠
  - 当前边界：
    - 只做本地结构解析，不验证签名真伪
    - 不拉取 JWKS，不做 issuer 联网校验，不替代完整认证审计流程
- 选择这些工具的原因：
  - 它们都属于安全分析里高频、低耦合、离线即可完成的小动作
  - 更重的方法论任务仍应优先走任务模板和 skills，而不是把 Apps 页做成并行工作流平台
- Apps 页当前边界：
  - 内置安全工具会显示为 `Built-in security tool / 内置安全工具`
  - 用户导入或聊天生成的 app 会继续显示为 `Imported / custom app / 导入/自定义应用`
  - 当前页面仍只展示 Goose `apps` runtime 管理的应用，不等于把所有 MCP app 都做成统一商店
- 旧默认 `clock` / `chat` 不再作为默认内置安全 Goose app 保留：
  - 新 workspace/runtime 会 seed 新的 4 个安全工具
  - 旧默认缓存条目会在默认 app 同步时被清理
  - 当前版本还会对同名内置工具执行受控刷新，确保 preview / packaged runtime 吃到最新内置 HTML，而不会长期停留在旧副本
- 当前一致性校验：
  - `node scripts/check-security-apps-runtime.mjs <GOOSE_PATH_ROOT>`
  - 会同时检查：
    - `data/apps/*.html`
    - `config/mcp-apps-cache/*.json`
  - 预期只保留这 4 个安全内置工具，不再保留 `clock` / `chat`
- 当前最小可视验收：

  ```bash
  ./scripts/run-security-visual-apps-smoke.sh
  ```

  这条 smoke 会在 repo preview 下确认：

  - `Apps` 页能看到 4 个内置安全工具
  - 不再显示旧默认 `clock` / `chat`
  - 至少一个内置工具可以从 Apps 页直接打开

## Goal 6 安全入口 smoke 检查表

完成上面的本机预览后，至少手工确认一次：

1. Launcher 快速入口可见 6 个安全任务：
   - 漏洞研判
   - 告警分析
   - IOC 研判
   - 网页调查
   - 报告生成
   - 业务逻辑排查（WooYun-style）
2. 任务模板页的“已保存任务模板”列表可见 6 个内置安全任务模板。
3. `security-vuln-triage`、`alert-investigation`、`ioc-analysis`、`web-investigation`、`report-writing`、`wooyun-legacy` 都沿 Goose 现有 `recipeId` 路径启动。
4. 如果 Launcher 某个安全任务显示 `Preview`，含义是当前工作目录缺少对应 recipe runtime，入口会回退到技能提示，而不是桌面新增了并行执行层。
5. Launcher 任务卡片里的 `主路径 / Primary path` 需要与启动行为一致：
   - recipe-backed 且 runtime 存在时，主路径为 recipe runtime
   - recipe 缺失 fallback 时，主路径会退回到 mapped skill prompt
6. 当前 Goose UI 仍不能显式证明“某个 skill 已被本次会话实际加载”；`技能映射 / Skill mapping` 只代表推荐方法论，不代表已确认的运行时遥测。

## Goal 9 任务与扩展联动 smoke 检查表

继续至少手工确认一次：

1. Launcher 安全任务区可见扩展状态总览。
   - `Browser Assist` / `Threat Intel` 标记为 `Local preview`
   - `AiseeSec` 标记为 `Blocked`
   - `Security Gateway` 标记为 `Disabled stub`
2. Launcher 安全任务卡片可见 `Recommended extensions` 标签区。
3. 任务模板页继续走 Goose 原生已保存任务模板列表，不再额外渲染安全任务区或 `Open Extensions` 按钮。
4. 扩展启用仍通过现有 `Settings -> Extensions` 入口完成，而不是新页面或并行平台层。
5. 任务仍旧通过 Goose 现有 `recipeId / starter prompt / skill hint` 路径启动，不因扩展推荐而改变运行时。

## Goal 10 可视回归前提

- `Computer Use` 需要当前 macOS 会话处于解锁状态
- repo 这份 `收到` Electron 窗口需要是当前前台窗口
- 首选先运行 `./scripts/start-security-preview.sh`
- 如果前台不是 `收到`，再执行 `./scripts/focus-security-preview-window.sh`

当前仓库内的最小可视回归建议顺序：

1. `./scripts/start-security-preview.sh`
2. `./scripts/focus-security-preview-window.sh`（如果窗口没有自动置前）
3. 用 Computer Use 检查：
   - Launcher 安全任务入口
   - 任务模板页已保存任务模板列表中的 6 个内置安全任务模板
   - 至少一条内置安全任务模板可点击
   - 推荐扩展状态可见

   注意：

   - Computer Use 应优先使用当前仓库这份 Electron app 的绝对路径
   - 不要用模糊 app 名 `Electron` 做验证；当前 macOS 会话里它可能命中别的仓库或别的 Electron `default_app`

如果本机 `Computer Use` 仍受桌面无障碍或会话状态限制，仓库内的替代校验仍然是：

- `./scripts/run-security-visual-smoke.sh`
- 这条链会用 Electron + Playwright 启动当前 repo 里的 收到，校验：
  - Launcher 安全任务入口可见
  - 任务模板页已保存任务模板列表可见
  - 至少一条内置安全任务模板可打开新窗口
  - 推荐扩展状态可见
- recipe-backed 入口的真实运行时 id 仍沿用 Goose 当前 `/recipes/list` 返回的 manifest hash；desktop 只在本地把 recipe 文件 stem 映射到该 id，不引入并行 recipe loader
- `pnpm --dir ui/desktop exec vitest run src/components/LauncherView.test.tsx src/components/recipes/RecipesView.test.tsx`
- 这条替代链覆盖任务卡片、zh-CN 文案、recipe 启动映射、主路径提示、skill telemetry 边界和扩展状态，但不等价于真实桌面窗口点击

## Preview backend helper smoke

如果你只想确认 preview backend helper 本身可用，而不是直接打开桌面窗口，可以执行：

```bash
./scripts/check-security-preview-backend.sh
```

这条检查会验证：

- repo 自带 preview backend helper 能选到 repo 内 `goosed`
- backend 能监听本机端口
- `https://127.0.0.1:$GOOSE_PORT/status` 在当前 secret 下可达
- backend 当前读取到的 `GOOSE_PROVIDER` 是 `openai`
- backend 当前读取到的 `OPENAI_BASE_URL` 是 `https://tokenhub.tencentmaas.com/plan/v3`
- backend 当前读取到的 `GOOSE_TELEMETRY_ENABLED` 是 `true`
- backend 当前读取到的 `GOOSE_POSTHOG_API_HOST` 是 `https://us.i.posthog.com`
- backend 当前读取到了 `GOOSE_POSTHOG_PROJECT_API_KEY`

如果你还要确认桌面同款聊天链路已经能真正发消息并拿到回复，可以继续执行：

```bash
./scripts/check-security-preview-chat.sh
```

这条检查会沿用桌面当前真实链路：

- `POST /agent/start`
- `GET /sessions/{id}/events`
- `POST /sessions/{id}/reply`

并验证至少一条消息能通过当前 Token Plan 默认接线拿到回复。

如果你还要做“真实桌面窗口里发消息拿到回复”的可视回归，可以继续执行：

```bash
./scripts/run-security-visual-chat-smoke.sh
```

这条检查会用 Electron + Playwright 拉起当前 repo 的 desktop，并验证：

- chat 输入框可见
- 至少一条真实消息能在桌面界面里发出
- assistant 最终返回 `pong` 风格确认回复
- 不再出现 `Provider not set`
- 不再出现 `Authentication failed`
- 不再出现旧的 `gpt-5.3-codex` 错误模型回退

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

- `ui/desktop/out/收到-darwin-arm64/`
- `ui/desktop/out/收到-darwin-x64/`

当前 packaged app 的 Goose-first 运行时边界是：

- 打包产物仍只使用包内 `Contents/Resources/bin/goosed`
- 不接受外部 `Goose.app` 自带的 `goosed`
- packaged app 启动时会把 bundled `security-cn` 里的安全 skills / recipes 以“只补缺、不覆盖用户修改”的方式种到当前工作目录：
  - `.agents/skills/`
  - `.goose/recipes/`
- 这一步是为了把发行源目录接回 Goose 当前原生 runtime 入口，不新增 skill loader 或并行 recipe engine
- 如果当前工作目录里的 skill / recipe 与 bundled 源发生缺失或漂移，Launcher 和任务模板页会直接显示 runtime attention warning
  - repo preview 可执行 `node scripts/sync-security-runtime-assets.mjs`
  - packaged preview 建议回到官方入口重新启动到目标工作目录

Skills 管理页 V1 的当前边界：

- `Skills` 页分成两组：
  - `Bundled Skills`
  - `My Skills / Local Skills`
- `Bundled Skills` 展示：
  - Goose builtin skills
  - bundled 收到 安全 skills
- `My Skills / Local Skills` 只展示当前项目 `<working-dir>/.agents/skills/` 下受控安装的本地 skill
- 导入范围固定为当前项目：
  - skill 文件夹
  - zip 包
  - 最终安装目录名取自 `SKILL.md` frontmatter 的 `name`
- 如果本地 skill 覆盖了 bundled security skill：
  - bundled 条目会显示 `Overridden locally`
  - 可执行 `Restore bundled version`
- 仍不开放 `.claude/skills`、插件缓存或其他非受控目录到这个管理页

当前本地 unsigned bundle 还额外固定了 3 个安装态边界：

- `bundle:default` / `bundle:intel` 会显式把 `GOOSE_DESKTOP_SIGN` 默认压成 `false`
  - 避免宿主 shell 里残留的 `APPLE_TEAM_ID` 等环境变量把本地 preview 误判成 signed build
- 本地 unsigned bundle 会把 `GOOSE_DISABLE_KEYRING=1` 写进 app 的 `LSEnvironment`
  - 目的是避免首次安装预览时触发 `收到 Key` 相关的钥匙串恢复/查找弹窗
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
xattr -dr com.apple.quarantine "/path/to/收到.app"
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
  - 如果这里失败，属于 Apple 环境 / 发布条件 blocker，不是 收到 本地 preview 架构 blocker

拿到 Apple secrets 的执行者，现应直接按
[`signed-release-runbook.md`](./signed-release-runbook.md)
执行：

- 优先跑 `Manual Desktop Bundle` 的 signed 演练
- 查看 arm64 / x64 的 release evidence artifact 与 job summary
- 只有 signed 演练通过后，再推进正式 `release.yml` tag 路径

如果执行者需要带命令的交付面板，而不是只看说明文档，请直接打开：

- [`signed-release-handoff-panel.md`](./signed-release-handoff-panel.md)

## 安装后首启 UX 检查表

对本地安装预览或 signed 候选包，至少手工确认一次：

1. Finder / Launchpad 中 app 名称显示为 `收到`。
2. 启动图标和应用图标正常，没有回退成默认占位。
3. 首次进入时默认文案是中文口径，而不是英文 onboarding。
4. Settings 中 provider / model 默认接线与 `distro/security-cn/config/desktop-env.example`、`model-catalog.json` 一致。
5. 任务模板页“已保存任务模板”列表仍能看到 6 个内置安全任务模板。
6. `漏洞研判`、`告警分析`、`IOC 研判`、`网页调查`、`报告生成`、`业务逻辑排查（WooYun-style）` 都仍保留 recipe-backed 链路。
8. Extensions 视图里的推荐安全扩展状态没有回退。

当前仓库内自动化对这份 checklist 的覆盖边界是：

- 自动化已覆盖：
  - app metadata / bundle id / signing mode / zip 解包
  - packaged app 可启动
  - bundle 内 `goosed` 与 `security-cn` 资源存在
  - Launcher 安全任务入口与任务模板页已保存任务模板列表
- 仍需手工确认：
  - Finder 图标观感
  - 首次进入时的中文文案观感
  - 安装后首屏的 provider/model 体验是否符合预期

如果你要做安装态 smoke，执行：

```bash
source bin/activate-hermit
./scripts/run-security-packaged-smoke.sh
```

这条检查会验证：

- packaged app 可拉起并保持运行
- startup diagnostics 显示 backend 来自包内 `Contents/Resources/bin/goosed`
- local-preview 会话在启动后的延迟窗口内不会继续命中 upstream `latest-mac.yml` / GitHub release 更新检查
- `/status` 健康检查已经通过
- `POST /agent/start` + `GET /sessions/{id}/events` + `POST /sessions/{id}/reply` 能通过 packaged 默认 Token Plan 链路拿到真实回复
- 当前工作目录下的 `.agents/skills` 与 `.goose/recipes` 已被正确补齐
- packaged smoke 工作目录会临时复制当前仓库根目录 `init-config.yaml`，以便包内 backend 按 Goose 原生方式读取 `OPENAI_BASE_URL` 和后端 secret

当前这条 packaged smoke 额外固定了两个边界：

- 不再通过改写 `HOME` 来做隔离，避免触发 macOS 的 `收到 Key` 钥匙串恢复弹窗
- unsigned 本地 bundle 默认不再要求 Electron 把本地 cookie/storage key 写入 macOS Keychain；signed build 仍保留该能力
- bundle 前会先停掉当前仓库 `out/.../*.app` 下仍在运行的 packaged 实例，避免重打包后留下“应用程序已不能再打开”的失效 app 进程
- 默认会先重打最新 `bundle:default`，避免 smoke 误复用旧 `.app`；如果你明确知道 bundle 已是最新产物，才显式加 `SECURITY_PACKAGED_SKIP_REBUILD=1`
- 对这类“靠 shell 环境变量注入隔离路径”的 local-preview bundle，不建议把 Computer Use 当成官方验收入口
  - 原因是 macOS 可能经由 LaunchServices 重新拉起同 bundle id 的 `.app`，把 `GOOSE_USER_DATA_DIR`、`GOOSE_PATH_ROOT` 这类 shell 注入环境丢掉，最后看到的不是你刚才那次 smoke 的 app 进程
- 当前 packaged 聊天回归的官方验证仍以 `./scripts/run-security-packaged-smoke.sh` 为准

关于更新链路，当前边界也固定为：

- `local-preview` / `packaged-preview` 默认不检查正式 release 更新
- Settings 里的 Updates 区域会明确提示：
  - 当前是本地预览包
  - 不会去检查 signed release 更新
  - 真正的正式更新链路只保留给 signed release 包
- 这样做是为了避免本地预览包继续打到 upstream `latest-mac.yml` 或 GitHub Releases，造成 404 / rate-limit / “更新失败” 噪音
- signed release 的 updater 边界不在这条 preview 入口里验证，仍按 signed bundle / notarization 流程单独验收

如果你要把 packaged local-preview 真正打开给人点，而不是只做 smoke，官方启动入口改为：

```bash
source bin/activate-hermit
./scripts/start-security-packaged-preview.sh
```

或者：

```bash
pnpm --dir ui/desktop run start:packaged-preview
```

这条入口会：

- 必要时先重打最新 `bundle:default`
- 停掉当前仓库这份 packaged preview 的旧实例
- 显式注入 `GOOSE_USER_DATA_DIR`、`GOOSE_PATH_ROOT`、`GOOSE_SERVER__SECRET_KEY`、`GOOSE_LOCAL_PREVIEW_BUNDLE=1`
- 把当前仓库根目录 `init-config.yaml` 复制到 packaged preview 工作目录
- 直接拉起 `.app/Contents/MacOS/收到`
- 等到 startup diagnostics 成功后再返回

当前 packaged local-preview 的支持边界明确分成 3 类：

1. 官方支持：
   - `./scripts/start-security-packaged-preview.sh`
   - `./scripts/run-security-packaged-smoke.sh`
2. 开发排障可用，但不作为官方验收：
   - shell 里手工直拉 `.app/Contents/MacOS/...`，前提是你自己带齐 `GOOSE_USER_DATA_DIR`、`GOOSE_PATH_ROOT`、`GOOSE_SERVER__SECRET_KEY` 和 `--dir`
3. 非官方入口，不承诺隔离态一致：
   - Finder / LaunchServices 双击 `.app`
   - Computer Use 直接按 `.app` 抓起或重拉起窗口

原因：

- Finder / LaunchServices 双击不会保留你 shell 临时注入的 `GOOSE_USER_DATA_DIR`、`GOOSE_PATH_ROOT`
- Computer Use 在当前工具态里也可能让 macOS 重新按 bundle id 拉起 `.app`
- 这会让 local-preview 包回落到“没有脚本注入隔离 env”的启动路径
- 本轮代码已补上 packaged local-preview 的 app 内 fallback 隔离目录，能减少回到全局默认状态的漂移
- 但它仍不能等价替代官方 wrapper 入口，因为 Finder/LaunchServices 不会自动补 `--dir` 和本地 preview secret

当前 desktop 里也会直接提示这条边界：

- 如果 packaged local-preview 不是通过官方 wrapper 拉起，而是落到了 fallback 启动路径：
  - 聊天页
  - onboarding
  - provider settings
  会出现一条黄色 warning，明确说明：
  - 当前会话正在使用 fallback 本地状态
  - 看到的聊天记录、模型或工作目录可能与官方隔离预览不一致
  - 建议回到仓库根目录，使用 `./scripts/start-security-packaged-preview.sh` 或 `pnpm --dir ui/desktop run start:packaged-preview`
- 这条 warning 是用户感知收口，不代表 Finder / LaunchServices 已进入官方支持范围

## 不做的事

- 不新建并行 runtime
- 不新建并行 memory system
- 不新建并行 tool scheduler
- 不做 gateway
- 不做 LiteLLM
- 不做 AGS
- 不做在线 marketplace
- 不做企业后台
