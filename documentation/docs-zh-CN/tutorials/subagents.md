---
title: 使用 Subagents
description: 拉起一组 subagents，协作构建一个完整可用的应用
image: /assets/images/tutorial-using-subagents-ef265627024db73e73d80e5799ed0c1a.png
---

<div style={{ display: "none" }}>
![](/img/tutorial-using-subagents.png)
</div>

这篇教程会带你体验如何拉起一组 AI [subagents](/zh-CN/docs/guides/subagents)，并引导它们像一个完整的软件团队那样协作，最终做出一个可以运行的应用。

你将构建 **AI BriefMe**，一个可以围绕任意主题生成结构化高管简报风格 briefing 的应用。

你会让 goose 负责协调一整套 subagents 软件团队：

- 🧠 **Planner**：定义产品方向和 MVP 范围
- 📋 **Project Manager**：拆任务并协调执行
- 🏗️ **Architect**：搭建项目结构和技术栈
- 💻 **Frontend / Backend Developers**：实现界面和 API 逻辑
- 🧪 **QA**：编写测试并标记 bug 或上线风险
- 📝 **Tech Writer**：编写安装、使用和 API 文档

完成这一轮之后，你不仅会得到一个可以运行的原型，还会更清楚地理解如何把 AI agents 用在更真实的软件协作流程里。

## 准备工作

1. [安装 goose](/zh-CN/docs/getting-started/installation)
2. 在 goose 中选择你的工作目录。建议使用一个新建目录来完成这次练习。
3. 把下面内容加入 [`.goosehints`](/zh-CN/docs/guides/context-engineering/using-goosehints)：

```plaintext
Create apps in html, javascript, and css when possible.
NEVER run blocking server commands (node server.js, npm start, etc.) - provide commands for user to run separately.
```

4. （可选）安装 [goose docs 扩展](/zh-CN/docs/mcp/goose-docs-mcp)，这样当你需要让 goose 解释自身行为时会更方便。

## 任务流程

你要做的是通过一组 subagents 来构建一个 AI 驱动的 briefing 应用。每个 agent 都有清晰的角色分工。你的关键任务不是手写所有内容，而是想办法把 prompt 提给 goose，让它正确委派工作。

> 🛟 如果卡住了，可以参考下面每个阶段附带的 prompt 示例。

---

### 1. 🧠 Planner

先让 Planner 定义产品愿景和范围。Planner 需要回答：

- 这个 app 是什么
- 它面向谁
- 它解决什么问题
- MVP 版本必须包含哪些核心能力

这一阶段的输出应该是一份清晰的产品定义，而不是代码。

<details>
  <summary>Planner Agent Prompt</summary>

  ```md
  你是 Planner agent，当前正在参加一个用 goose 和 subagents 进行 AI 应用实战构建的环节。我们要 *立刻开始* 做 MVP。

  应用名叫 **AI BriefMe**。它会围绕任意主题生成一份简报。用户输入类似 “Apple earnings” 或 “AI in DevOps” 这样的主题，应用需要返回：
  - 一个标题
  - 今天的日期
  - 2–3 条要点摘要
  - （可选）如果主题偏技术，可附带代码片段或图表

  你正在和一组 subagents 协作：PM、Architect、Frontend Dev、Backend Dev、QA、Tech Writer，他们会马上开始执行你的计划。

  请输出一份简短、聚焦的 **Markdown 文件（`plan.md`）**，内容包括：
  - MVP 的目标
  - 仅限 40 分钟内可以完成的功能
  - 任何有帮助的设计考虑

  ✅ DO: 保持精简且可执行
  ❌ DON'T: 不要加入长期功能，例如邮件发送、用户账户、dashboard、analytics、个性化、移动端优化，或 8 周路线图
  ```
</details>

---

### 2. 📋 Project Manager

接下来让 Project Manager 把产品拆成开发任务。输出应当：

- 定义需要哪些角色（例如 frontend、backend）
- 列出具体任务并分派给对应角色
- 标明哪些任务可以并行，哪些必须串行

<details>
  <summary>PM Agent Prompt</summary>

  ```md
  你是 PM agent。Planner 刚刚为一个名为 "AI BriefMe" 的应用创建了 `plan.md`，目标是在 1 小时内完成可运行版本。

  你的工作是：
  - 把工作拆分给各个 subagent：Architect、Backend Dev、Frontend Dev、QA、Tech Writer
  - 按 agent 分组整理任务
  - 判断哪些工作可以并行执行，哪些必须串行推进
  - 用 Markdown 输出任务拆解，并保存为 `project_board.md`

  请保持现实和简洁：这是一次冲刺，不是长期 roadmap。
  ```
</details>

---

### 3. 🏗️ Architect

让 Architect 负责技术方案和脚手架。它应该：

- 选择技术栈（前端、后端框架及需要的库）
- 描述目录结构和文件组织方式
- 标记需要安装的内容（必要时附上安装命令）

<details>
  <summary>Architect Agent Prompt</summary>

  ```md
  你是 Architect。请基于项目计划和 `project_board.md` 搭建项目脚手架。

  你需要完成：
  - 创建目录结构和所有占位文件（例如 `index.html`、`server.js`、`style.css` 等）
  - 生成一个 `package.json`，其中依赖包含 `express`、`cors` 和 `child_process`
  - 添加 `.gitignore`，忽略 `node_modules` 和任何临时文件
  - 用 Markdown 定义 `/api/briefing` 端点的 API contract

  ✅ 不要包含或引用任何 API key
  ✅ 不要安装任何包，只做脚手架
  ✅ 在结尾列出输出的文件和目录
  ```
</details>

---

### 4. 💻 Frontend + Backend Developers（并行）

现在并行拉起两个开发 subagents 来构建核心应用。一个负责 Express server 和后端逻辑，另一个负责 UI 和表单交互。重点是：goose 需要**同时执行**这两个 agent，而不是先后串行。

- 使用 Architect 产出的文件结构和 API contract
- Backend 用 Headless goose 实现 API 逻辑
- Frontend 负责构建响应式 UI 并调用 API
- 确保两个 agent 不去写同一组文件

<details>
  <summary>Dev Agents Prompt</summary>

  ```md
  请使用 **parallel execution** 同时运行两个 subagents：

  - 一个 🛠️ Backend Developer
  - 一个 💻 Frontend Developer

  两个 agent 必须并行工作，而不是串行地去构建 AI BriefMe MVP。

  🛠️ **Backend Developer** 需要：
  - 使用 Express 实现 `server.js`
  - 添加 POST `/api/briefing` 端点，接收 `{ "topic": "string" }`
  - 使用 **Headless goose** 生成摘要：
    - `goose run -t "YOUR_PROMPT_HERE" --quiet --no-session --max-turns 1`
  - 使用 `child_process.spawn()` 而不是 `exec()`
  - 清洗返回内容：去掉 ANSI 色彩码、markdown code block，并提取 JSON
  - 处理超时（最长 60 秒）和错误
  - 使用 `express.static` 提供前端静态文件
  - 添加 CORS
  - 不要要求任何 API key

  💻 **Frontend Developer** 需要：
  - 创建 `index.html`、`style.css` 和 `script.js`
  - 做一个输入 topic 的表单
  - 调用 `/api/briefing` 并展示结果
  - 处理 loading 状态和错误提示
  - 加入 copy-to-clipboard 按钮
  - 做到移动端友好
  - 不要干扰后端文件

  ⚠️ Important: 两个 agent 不能写同一个文件。请保持工作隔离。
  ```
</details>

---

### 5. 🧪📝 QA + Tech Writer（并行）

开发完成后，再拉起最后两个 subagents：**QA Engineer** 和 **Tech Writer**。它们会同时评估应用质量并补全文档。你的任务是把 prompt 写清楚，让它们在协作时不重叠、不重复。

#### QA Agent 任务：

- 为 `/api/briefing` 端点编写单元测试，测试框架可用 Jest
- 创建 `QA_NOTES.md`：
  - 标出 bug 或边界情况
  - 指出上线阻塞项（例如安全、错误处理）
  - 给出下一步提高可靠性的建议

#### Tech Writer 任务：

- 与 QA 协作，了解当前应用状态
- 创建 `README.md`，包含：
  - 应用做什么（用自然语言说明）
  - 如何安装和运行
  - API 使用示例

⚠️ Important:

- **QA 不应手动启动 server**，只编写测试文件并 mock 交互

<details>
  <summary>QA Agent + 📝 Tech Writer Agent Prompt（并行）</summary>

  ```md
  开发阶段已经完成。现在进入质量保障和文档阶段。

  请使用 **parallel execution** 同时运行两个 subagents：

  - 🧪 一个 **QA Agent**，负责：
    - 为 `/api/briefing` 端点编写单元测试 `tests/briefing.test.js`，使用 Jest
    - 在测试文件顶部使用 `jest.mock('child_process')` mock `child_process`
    - 创建一个简单 mock，返回假数据，而不是调用真实 goose CLI
    - 断言响应中包含：`title`、`date` 和 2–3 条 `takeaways`
    - 覆盖以下测试场景：
      - topic 输入合法
      - topic 缺失或非法
      - goose CLI 超时或报错
    - **不要手动启动或运行 server。** 只写测试文件。
    - **不要执行 `npm test` 或运行任何测试。** 只创建测试文件。
    - 在 `QA_NOTES.md` 中保存完整 QA 分析报告，包含：
      - Critical issues
      - Security or performance gaps
      - Recommendations for production readiness
    - **文件全部创建完成后，立即输出：`QA Agent Sign-off: ✅ COMPLETE` 并结束。**

  - 📝 一个 **Tech Writer Agent**，负责：
    - 创建 `README.md`，内容包含：
      - Project overview
      - How to run the app locally
      - API endpoint documentation
      - Example request/response
      - Troubleshooting section
    - **文档完成后，立即输出：`Tech Writer Sign-off: ✅ COMPLETE` 并结束。**

  两个 agent 必须并行执行，而不是串行。
  ```
</details>

---

## 测试你完成的应用

当所有 agents 都完成工作后，你应该已经得到一个可运行的原型。下面是实际启动和验证它的方式：

### 第 1 步：安装依赖

```bash
cd your-project-folder
npm install
```

### 第 2 步：启动 Server

**Important**：请在**单独的终端窗口**里运行，不要在 goose 里运行：

```bash
npm start
```

你应该会看到：

```text
AI BriefMe server running on port 3000
Health check: http://localhost:3000/health
Briefing endpoint: http://localhost:3000/api/briefing
```

### 第 3 步：打开应用

在浏览器中访问：

```text
http://localhost:3000/
```

**注意**：访问根路径 `/`，不要访问 `/ai-briefme/`

### 第 4 步：测试它

1. 输入类似 `JavaScript frameworks` 或 `climate change` 这样的主题
2. 点击 `Get Briefing`
3. 观察 AI 自动生成 briefing

你应该能看到：

- 一个干净且响应式的界面
- 包含标题、日期和 takeaways 的 AI 生成 briefing
- 针对技术主题返回的代码示例
- copy-to-clipboard 功能

:::tip 保持 Server 持续运行
- **不要关闭**运行 server 的那个终端
- **不要在 goose 里启动 server**，否则会卡住
- 需要停止时，在 server 所在终端按 `Ctrl+C`
- 如果你想让 goose 修 bug 或加功能，等它完成后再重启 server
:::

**恭喜，你已经用 goose subagents 做出了一个全栈 AI 应用。** 🎉

:::warning
不要期待这个应用已经达到生产可用水准。这个 workshop 展示的是 goose 如何加速原型开发，但最终的判断、质量把关和打磨仍然需要人来负责。
:::

---

## 故障排查

### Cannot POST /api/briefing（404 Error）

**原因**：路由没有正确注册，或者 server 修改后没有重启

**解决方式：**

1. **先停止 server**（终端里按 `Ctrl+C`）
2. **重新启动 server**：`node server.js`
3. **用 curl 测试接口**：

   ```bash
   curl -X POST http://localhost:3000/api/briefing -H "Content-Type: application/json" -d '{"topic":"test"}'
   ```

4. **预期返回应该是 JSON，不是 HTML**

---

### Unexpected token

**原因**：前端拿到的是 HTML，而不是 JSON（通常是 404 页面）

**解决方式：**

1. **先直接测试 API**：

   ```bash
   curl -X POST http://localhost:3000/api/briefing -H "Content-Type: application/json" -d '{"topic":"test"}'
   ```

2. **如果返回 HTML**：说明 API 路由本身有问题（回到上一条排查）
3. **如果返回 JSON**：打开浏览器的 Network 面板，检查前端实际请求的是哪个 URL
4. **常见修复**：确认你访问的是 `http://localhost:3000/`，而不是 `/ai-briefme/`

---

### Process Timeout

**原因**：goose 执行过慢，或者命令卡住了

**解决方式：**

1. **检查 goose 命令参数**：

   ```javascript
   ['run', '-t', prompt, '--quiet', '--no-session', '--max-turns', '1']
   ```

2. **手动测试 goose**：

   ```bash
   goose run -t "Return JSON: {\"test\": \"value\"}" --quiet --no-session --max-turns 1
   ```

3. **如果手测正常**：重点检查 `spawn()` 实现
4. **如果手测也卡住**：先把 prompt 简化，再逐步增加复杂度

---

### JSON Parsing Errors

**原因**：goose 返回的内容里带有颜色码或 markdown 包装

**解决方式：**

1. 在 `JSON.parse()` 前加入清洗逻辑：

   ```javascript
   // Remove ANSI color codes
   jsonString = jsonString.replace(/\x1b\[[0-9;]*m/g, '');
   // Remove markdown formatting
   jsonString = jsonString.replace(/```json\s*/, '').replace(/```\s*$/, '');
   jsonString = jsonString.replace(/```\s*/, '');
   ```

2. 加一点调试日志，看看实际收到了什么：

   ```javascript
   console.log('Raw goose response:', aiResponse);
   console.log('Cleaned JSON string:', jsonString);
   ```

---

### Port Already in Use (`EADDRINUSE`)

**解决方式：**

1. **先找出谁占用了 3000 端口**：

   ```bash
   lsof -ti:3000
   ```

2. **杀掉对应进程**：

   ```bash
   lsof -ti:3000 | xargs kill -9
   ```

3. **或者换一个端口**：

   ```javascript
   const PORT = process.env.PORT || 3001;
   ```

---

### App 显示空白页

**解决方式：**

1. **先确认访问 URL 正确**：`http://localhost:3000/`，不要用 `/ai-briefme/`
2. **检查浏览器 console** 是否有 JavaScript 错误
3. **确认静态文件已经正确托管**：

   ```javascript
   app.use(express.static(__dirname));
   ```

4. **逐个检查静态文件是否能访问**：
   - `http://localhost:3000/index.html`
   - `http://localhost:3000/style.css`
   - `http://localhost:3000/script.js`
