# Skills source directory

这里保存安全 skill 的发行源目录。

Goose 当前不会自动从 `distro/security-cn/skills/` 发现 skills。

Goal 5 的本机预览接线方式：

- 发行源目录：`distro/security-cn/skills/*/`
- 运行时镜像：项目 `.agents/skills/*/`
- 同步脚本：`scripts/sync-security-runtime-assets.mjs`
- packaged preview seed：当前工作目录 `.agents/skills/*/`
  - 由 desktop `securityRuntimeBootstrap` 在官方 packaged preview 启动链里补齐缺失文件
  - 不新增 skill loader，仍然依赖 Goose 原生 `.agents/skills/` 发现逻辑
  - packaged working dir 默认只补缺，不强制覆盖；如与 bundled 源漂移，桌面安全任务入口会给出 runtime warning

不新增 skill loader，也不修改 Goose 的发现逻辑。

当前桌面可见性边界：

- Security Goose 的技能页面和 slash-command 技能列表只展示两类技能：
  - Goose 自带 builtin skills
  - 随发行版附带并同步到 `.agents/skills/` 的 Security Goose 安全 skills
- 当前项目通过 Skills 管理页导入到 `<working-dir>/.agents/skills/` 的受控本地 skill 也会作为“当前项目可见 skill”纳入桌面管理视图，并在 slash-command 下一次重新打开时按同一受控范围参与显示
- 其他 `.claude/skills/`、插件缓存或用户私有 skill 目录仍可能被 Goose 运行时发现，但不会作为 Security Goose 桌面默认可见技能入口展示
- 这是产品层可见性收口，不是新的 skill loader，也不改变 Goose 原生技能目录机制

Skills 管理面 V1：

- `Bundled Skills`
  - 展示 Goose builtin skills
  - 展示 `distro/security-cn/skills/*` 作为 bundled 基线的安全 skills
  - 如果当前项目 runtime 缺失 bundled 副本，会标记为 `Missing runtime`
  - 如果当前项目 runtime 覆盖了 bundled 副本，会标记为 `Overridden locally`
  - 对 bundled 副本缺失或已覆盖的条目，可执行 `Restore bundled version`
- `My Skills / Local Skills`
  - 只展示当前项目 `<working-dir>/.agents/skills/*` 下的受控本地 skill
  - 普通本地 skill 记为 `local-custom`
  - 与 bundled 同名但内容已替换的本地 skill 记为 `local-override`
  - 缺少有效 `SKILL.md` frontmatter `name` 或目录不匹配的条目记为 `invalid`
- 导入范围固定为当前项目：
  - skill 文件夹
  - zip 包
  - 安装目录名取自 `SKILL.md` frontmatter 的 `name`
- 覆盖语义：
  - 允许当前项目本地 skill 覆盖 bundled security skill
  - packaged preview 重启后，本地 override 继续保留，不会被自动恢复
- 只有显式执行 `Restore bundled version` 才会把 bundled 源重新复制回当前项目 runtime

Skills 管理面 V2：

- `Skills` 页面现在补齐三类只读信息，不新增 loader：
  - 技能详情：可查看技能名称、描述、skill 文件夹位置，以及去掉 frontmatter 后的 `SKILL.md` 正文
  - 运行时诊断：仅在缺失 / 覆盖 / 无效时显示原因和修复建议
  - 当前项目操作反馈：导入、删除、恢复 bundled 后会给出明确成功提示
- `Skills` 页顶部不再展示“当前项目技能统计 / runtime 数量”这类实现者视角信息，只保留“这里会显示哪些技能”的用户视角说明
- 文件夹查看边界：
  - 详情页只提供“在 Finder 中打开 skill 文件夹”
  - bundled security skill 不再额外暴露 bundled 源目录入口
  - builtin Goose skill 没有可恢复的当前项目 skill 包，因此只展示说明，不伪装成本地可浏览目录
- invalid skill 会继续显示原始 invalid detail，并补一条最小修复建议：
  - 缺少 `SKILL.md`
  - 缺少或损坏 frontmatter
  - `name` 缺失
  - 目录名与 frontmatter `name` 不一致

当前方法论边界：

- Security Goose 当前 6 个安全任务都以 Goose 原生 recipe 作为主执行路径
- skill 主要承担方法论补充、风险边界参考和输出模板参考
- 当前 Goose UI 还不能显式证明某个 skill 已在会话里加载，因此不能把 skill 映射伪装成已观测的运行时信号
- 只有当当前工作目录缺少 recipe runtime 时，桌面入口才会 fallback 到 skill prompt

当前可见性边界：

- `Skills` 页展示的是当前产品允许用户浏览和管理的技能范围：
  - Goose builtin
  - bundled Security Goose 安全 skills
  - 当前项目受控本地 skills
- `/` 菜单展示的是当前 Goose runtime 这一轮真正发现到的 skill 命令
- 因此新导入的本地 skill 可能先出现在 `Skills` 页，再在重新打开会话后出现在 `/` 菜单
- 这是 Goose 原生 discoverability 与产品可见性收口之间的边界，不是新的 skill loader

附加约束：

- `wooyun-legacy` 只内置 Goose-native 包装 skill
- 上游参考资料采用 `CC BY-NC-SA 4.0`，不直接 vendoring 到 fork
- 如需本机增强版参考包，使用 `scripts/install-wooyun-legacy-skill.mjs` 安装到 `.agents/skills/wooyun-legacy/external/upstream/`
