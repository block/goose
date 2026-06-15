# Skills source directory

这里保存安全 skill 的发行源目录。

Goose 当前不会自动从 `distro/security-cn/skills/` 发现 skills。

Goal 5 的本机预览接线方式：

- 发行源目录：`distro/security-cn/skills/*/`
- 运行时镜像：项目 `.agents/skills/*/`
- 同步脚本：`scripts/sync-security-runtime-assets.mjs`

不新增 skill loader，也不修改 Goose 的发现逻辑。

附加约束：

- `wooyun-legacy` 只内置 Goose-native 包装 skill
- 上游参考资料采用 `CC BY-NC-SA 4.0`，不直接 vendoring 到 fork
- 如需本机增强版参考包，使用 `scripts/install-wooyun-legacy-skill.mjs` 安装到 `.agents/skills/wooyun-legacy/external/upstream/`
