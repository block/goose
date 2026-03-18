---
sidebar_position: 2
title: "调整工具输出详细程度"
sidebar_label: "调整工具输出"
description: "介绍如何控制工具输出的详细程度。"
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
Response Styles 用来控制 goose Desktop 聊天窗口中，工具调用过程应当如何展示。

修改方法：
1. 点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏。
2. 点击侧边栏中的 `Settings`。
3. 点击 `Chat`。
4. 在 `Response Styles` 中选择 `Detailed` 或 `Concise`。

- **Concise**（默认）
    - 工具调用默认折叠
    - 只显示 goose 使用了哪个工具
    - 更适合关注结果而不是细节的用户

- **Detailed**
    - 工具调用默认展开
    - 会显示工具调用过程及其返回细节
    - 更适合调试场景，或者想了解 goose 内部工作方式的用户

这个设置只会影响会话里工具调用的默认展开状态。无论你选哪种样式，之后都可以手动展开或折叠任意一次工具调用。

</TabItem>
  <TabItem value="cli" label="goose CLI">
在 goose CLI 中，你也可以控制工具输出的详细程度。

要调整工具输出，请运行：

```sh
goose configure
```

然后选择 `Adjust Tool Output`

```sh
┌   goose-configure 
│
◆  What would you like to configure?
│  ○ Configure Providers 
│  ○ Add Extension 
│  ○ Toggle Extensions 
│  ○ Remove Extension
// highlight-next-line
│  ● Adjust Tool Output (Show more or less tool output)
└  
```

接着选择一种模式：

```sh
┌   goose-configure 
│
◇  What would you like to configure?
│  Adjust Tool Output 
│
// highlight-start
◆  Which tool output would you like to show?
│  ○ High Importance 
│  ○ Medium Importance 
│  ○ All 
// highlight-end
└  
```

- **High Importance**
    - 只显示最重要的工具输出
    - 输出最精简

- **Medium Importance**
    - 显示中等重要度和高重要度的输出
    - 例如：文件写入操作的结果

- **All**
    - 显示所有工具输出
    - 例如：shell 命令输出
    - 信息最完整

### 切换参数截断

在活跃会话中，你还可以使用 `/r` slash command 来切换：是截断工具参数显示，还是完整显示：

```sh
Context: ●○○○○○○○○○ 5% (9695/200000 tokens)
( O)> /r
✓ Full tool output enabled - tool parameters will no longer be truncated
```

当你需要看到完整文件路径、URL 或命令参数时，这会非常有用。再次输入 `/r` 即可恢复为截断显示。
 </TabItem>
</Tabs>
