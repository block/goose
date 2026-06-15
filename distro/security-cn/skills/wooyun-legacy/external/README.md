# wooyun-legacy external reference pack

这个目录只放 `wooyun-legacy` 的本地增强资料说明，不直接分发上游内容。

原因：

- 上游仓库：`https://github.com/tanweai/wooyun-legacy`
- 上游许可证：`CC BY-NC-SA 4.0`
- 上游许可证明确限制商业使用，并要求衍生内容保持相同许可证

因此，`security-goose` fork 只内置 Goose-native 的包装 skill，不把上游 `SKILL.md` 和 `references/*.md` 直接 vendoring 到发行源目录。

如需在本机预览中启用增强参考包：

```bash
node scripts/install-wooyun-legacy-skill.mjs /path/to/wooyun-legacy
```

安装目标：

- `.agents/skills/wooyun-legacy/external/upstream/UPSTREAM-SKILL.md`
- `.agents/skills/wooyun-legacy/external/upstream/UPSTREAM-README.md`
- `.agents/skills/wooyun-legacy/external/upstream/LICENSE`
- `.agents/skills/wooyun-legacy/external/upstream/references/*.md`

注意：

- 该安装脚本只把上游内容落到本地 runtime 目录，不回写 `distro/security-cn/`
- `.gitignore` 已默认忽略 `.agents/skills/wooyun-legacy/external/upstream/*`，避免把受限内容误提交进 fork
