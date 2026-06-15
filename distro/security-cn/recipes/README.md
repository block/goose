# Recipes source directory

这里保存安全 recipe 的发行源文件。

Goose 当前会从这些位置发现 recipes：

- 当前目录
- `GOOSE_RECIPE_PATH`
- `.goose/recipes/`
- `~/.config/goose/recipes/`

Goal 5 的本机预览接线方式：

- 发行源目录：`distro/security-cn/recipes/*.yaml.example`
- 运行时镜像：项目 `.goose/recipes/*.yaml`
- 同步脚本：`scripts/sync-security-runtime-assets.mjs`

这里选择 `.goose/recipes/`，因为它是 Goose 当前标准 recipe 库位置；本仓库只对
`.goose/recipes/*.yaml` 做了最小 git 跟踪例外，不扩展到其他 `.goose/` 运行时状态。
