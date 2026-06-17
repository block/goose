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
- packaged preview seed：当前工作目录 `.goose/recipes/*.yaml`
  - 由 desktop `securityRuntimeBootstrap` 在官方 packaged preview 启动链里补齐缺失 recipe 文件
  - 若 working dir 里的 recipe 与 bundled 源漂移，安全任务入口会明确标记为 runtime attention，而不是静默伪装成完全可用

这里选择 `.goose/recipes/`，因为它是 Goose 当前标准 recipe 库位置；本仓库只对
`.goose/recipes/*.yaml` 做了最小 git 跟踪例外，不扩展到其他 `.goose/` 运行时状态。

当前方法论边界：

- 6 个安全任务的主路径都是 Goose 原生 recipe runtime
- desktop 的 Launcher 快捷入口和任务模板页在启动这 6 个内置安全任务时，会附加同一份 starter prompt，用来固定输出字段、方法论边界和 skill 映射提示
- recipe 内 instructions 和 `message:` 活动仍是运行时主约束；starter prompt 只负责把同一套任务要求稳定送进会话起点
- skill 仅作为方法论补充和输出模板参考，不是当前 Goose UI 可确认的显式运行时遥测
- 如果当前 Goose 会话不能直接观测到 skill 已加载，应仍以 recipe 的流程和输出字段为准
