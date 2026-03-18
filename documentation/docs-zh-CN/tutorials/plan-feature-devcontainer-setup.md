---
title: "规划复杂任务"
description: "介绍如何用 goose 的 Plan 功能把复杂任务拆成可执行步骤。"
---

# 规划复杂任务

*把看似压垮人的任务，拆成系统化、可执行的步骤计划。*

当你用 goose 处理大而复杂的任务时，如果一开始并不清楚最佳做法，整个过程很容易让人觉得无从下手。我的一个典型经历是：要为自己发布的一个 [API 课程](https://github.com/LinkedInLearning/java-automated-api-testing-with-rest-assured-5989068) 搭建复杂的开发环境。Docker 配置、数据库初始化、devcontainer 设置、GitHub Codespaces 集成，这些部件必须严丝合缝地配合。只要漏掉一个配置项、写错一个依赖，整个流程就会偏离正轨。

本教程会演示如何使用 goose 的 [Plan 功能](/zh-CN/docs/guides/creating-plans)，把一个复杂的 devcontainer 搭建任务转换成系统化、可执行的路线图。你会看到如何先和 goose 一起做需求梳理，再把高层目标收敛成具体计划、实施 prompt 和验证清单。

## 你将学到什么

读完本教程后，你会知道如何：

- 使用 goose 的 Plan 功能，把复杂技术项目拆成可执行步骤
- 在对话里和 goose 一起做需求澄清与方案收敛
- 把高层目标变成细致、可执行的工作项
- 为实施过程补上验证清单和质量检查
- 按照清晰的 deliverables 系统执行计划

## 挑战：完整的 Codespaces 开发环境

我们的目标，是创建一个 GitHub 仓库，为 Rest Assured API 测试提供完整开发环境。当学生在 GitHub Codespaces 中打开仓库时，他们应该立刻拥有：

- **自动化安装**：不需要手工配置
- **API Server**：REST API 自动运行
- **数据库**：MySQL 自动初始化并带测试数据
- **开发工具**：Java 17、Docker 以及必要的 VS Code 扩展
- **一键启动**：通过 `docker-compose up -d` 拉起全部组件

这意味着要同时协调 Docker、MySQL、PHP、Java、devcontainers 和 GitHub Codespaces。

## Step 1：先做一轮需求脑暴

先别急着让 goose “直接开始做”，而是先通过一轮对话把需求讲清楚。

:::info Desktop 和 CLI 里的 Planning 差异
goose Desktop 没有像 CLI 那样的 `/plan` 关键词。在 Desktop 里，你需要明确告诉 goose：

```
"Hey goose, can you create a plan to set up my GitHub repo for Codespaces with automatic Docker startup? Please don't start the actual work yet."
```

而在 CLI 中，你可以直接进入 `/plan` 模式，goose 会先问澄清问题，再生成计划。除非你明确要求 goose Desktop “先创建计划”，否则它很可能会直接跳进实现。
:::

### 开始对话

**你：** _“我想搭一个 GitHub 仓库，用来做自动化 API 测试。学生应该能在 GitHub Codespaces 里直接打开仓库，不用额外配置就能开始写测试。你能帮我一起梳理需要准备什么吗？”_

**goose：** _“当然可以。为了把这个自动化环境设计清楚，我先问几个澄清问题：_

1. _你在测的 API 是什么类型？REST、GraphQL，还是其他？_
2. _学生会用什么测试框架？_
3. _你需要真实数据库和测试数据，还是 mock response 就够？_
4. _测试代码希望用什么语言写？_
5. _他们主要要测哪些 API endpoint 或功能？_”

### 收敛需求

**你：** _“这是一个 PHP 写的 REST API，学生会用 Java 里的 Rest Assured 来测试。需要真实的 MySQL 数据库和测试数据，好让他们练真实场景。这个 API 主要提供电商系统里的 products 和 categories 管理接口。”_

**goose：** _“很好，这样场景就清楚多了。我再确认几件事，确保设计方向正确：_

1. _你已经有现成的 PHP API 代码了吗，还是还要新建？_
2. _Java 需要哪个版本？_
3. _MySQL 里是否需要预填一些示例商品和分类数据？_
4. _是否希望预装特定的 VS Code 扩展？_
5. _你期望多快把这一套搭起来？_”

**你：** _“我已经有现成的 PHP API 代码，目前在 MAMP 中运行。Rest Assured 需要 Java 17。数据库需要示例数据。还请预装 Java 和 Docker 相关的 VS Code 扩展。”_

## Step 2：让 goose 生成详细计划

现在需求已经澄清得差不多了，就可以让 goose 输出完整计划。

### 请求计划

**你：** _“基于刚才的讨论，请为这个 GitHub 仓库搭建过程生成一份详细计划。我希望它按步骤拆清楚，并覆盖我们讨论过的技术要求。请把每一步的 deliverable 和验证方式也写出来。”_

### goose 生成的计划

goose 最终通常会输出类似下面这样的综合计划：

<details>
  <summary>GitHub Codespaces API 测试环境搭建计划</summary>

```markdown
# GitHub Codespaces API 测试环境搭建计划

## 项目概览
创建一个 GitHub 仓库，用于自动部署完整的 API 测试开发环境，包含 PHP API server、MySQL 数据库和 Java 开发工具。

## Phase 1：仓库结构与 API 集成

### 1.1 处理 API 文件
- **Deliverable**：整理好的 API 代码库结构
- **Tasks**：
  - 在仓库根目录放置 `/api-server/`
  - 按合理结构整理 PHP 文件：

    /api-server/
        category/
        config/
        objects/
        product/
        ApiTestingDB.sql

- **Validation**：所有 PHP 文件可访问，结构清晰

### 1.2 数据库 schema 与测试数据
- **Deliverable**：包含 schema 和样例数据的 SQL 文件
- **Tasks**：
  - 检查现有 `ApiTestingDB.sql`
  - 确认它既包含建表语句，也包含测试数据插入语句
  - 验证样例数据是否覆盖真实电商场景
- **Validation**：SQL 文件能无错误执行，并成功导入测试数据

## Phase 2：容器化

### 2.1 Docker 化 API Server
- **Deliverable**：`api-server/Dockerfile`
- **Tasks**：
  - 使用 `php:8.1-apache` 作为基础镜像
  - 安装所需 PHP 扩展（`pdo_mysql`）
  - 配置正确的文件权限和 Apache 设置

- **Example Configuration**：

  FROM php:8.1-apache
  RUN docker-php-ext-install pdo pdo_mysql
  COPY . /var/www/html/

- **Validation**：镜像能成功构建并正常提供 PHP 服务

### 2.2 数据库容器配置
- **Deliverable**：`docker-compose.yml` 中的 MySQL service 配置
- **Tasks**：
  - 配置 MySQL 5.7 容器和必要环境变量
  - 通过 volume 实现 schema 自动初始化
  - 配置 PHP 和 MySQL 之间的网络通信
- **Database Credentials**：
  | Setting  | Value |
  |----------|-------|
  | Host     | `mysql` |
  | Database | `api_db` |
  | Username | `user` |
  | Password | `password` |

- **Validation**：数据库可以自动启动并装载测试数据

### 2.3 服务编排
- **Deliverable**：完整的 `docker-compose.yml`
- **Tasks**：
  - 定义 API server 和 MySQL 两个服务
  - 配置正确的依赖关系
  - 配置对外端口映射
  - 配置数据库初始化所需的 volume
- **Example Configuration**：

  version: '3.8'
  services:
    mysql:
      image: mysql:5.7
      restart: always
      environment:
        MYSQL_ROOT_PASSWORD: password
        MYSQL_DATABASE: api_db
        MYSQL_USER: user
        MYSQL_PASSWORD: password
      ports:
        - "3306:3306"
      volumes:
        - ./api-server/sql:/docker-entrypoint-initdb.d

    api-server:
      build: ./api-server
      ports:
        - "8888:80"
      depends_on:
        - mysql

- **Validation**：两个服务都能正常启动并互相通信

## Phase 3：数据库连接配置

### 3.1 更新 PHP 数据库配置
- **Deliverable**：更新后的 `api-server/config/database.php`
- **Tasks**：
  - 把连接设置改成 Docker 网络环境
  - 将 host 从 `localhost` 改成 `mysql`
  - 确认凭证与 docker-compose 中保持一致
- **Validation**：PHP API 能成功连接 MySQL 容器

## Phase 4：Codespaces 集成

### 4.1 Devcontainer 配置
- **Deliverable**：`.devcontainer/devcontainer.json`
- **Tasks**：
  - 为 Java 17 开发创建 devcontainer 配置
  - 启用 Docker-in-Docker
  - 配置自动启动容器
  - 预装必要的 VS Code 扩展
- **Example Configuration**：

  {
    "name": "API Testing Environment",
    "image": "mcr.microsoft.com/devcontainers/java:17",
    "features": {
      "docker-in-docker": "latest"
    },
    "postCreateCommand": "docker-compose up -d",
    "customizations": {
      "vscode": {
        "extensions": [
          "vscjava.vscode-java-pack",
          "ms-azuretools.vscode-docker"
        ]
      }
    }
  }

- **Validation**：Codespace 启动后工具链全部可用

## Phase 5：测试与验证

### 5.1 API 访问验证
- **Tasks**：
  - 确认 API 可通过 `http://localhost:8888` 访问
  - 测试关键 endpoint（products、categories）
  - 确认数据库查询结果正确
- **Validation**：所有 API endpoint 都正确响应

### 5.2 Codespaces 集成测试
- **Tasks**：
  - 在 GitHub Codespaces 中打开仓库
  - 验证 Docker 自动启动
  - 测试 VS Code 扩展是否可用
  - 确认 Java 17 可用
- **Validation**：完整环境可直接供学生使用

## QA Checklist

### ✅ API Server 与数据库配置
- [ ] `/api-server/` 已放到仓库根目录
- [ ] category/、config/、objects/、product/ 等目录齐全
- [ ] `ApiTestingDB.sql` 含 schema 和测试数据
- [ ] `Dockerfile` 使用 `php:8.1-apache` 并安装 `pdo_mysql`
- [ ] `docker-compose.yml` 同时定义 MySQL 和 API 服务
- [ ] 服务依赖和端口映射正确

### ✅ 数据库配置
- [ ] PHP 数据库配置已使用 Docker 网络（`mysql` host）
- [ ] PHP 配置与 docker-compose 凭证一致
- [ ] 数据库能自动初始化测试数据

### ✅ Codespaces 配置
- [ ] 仓库根目录存在 `.devcontainer/`
- [ ] `devcontainer.json` 指定 Java 17
- [ ] Docker-in-Docker 已启用
- [ ] `postCreateCommand` 设置为 `docker-compose up -d`
- [ ] Java 和 Docker VS Code 扩展已配置

### ✅ 集成测试
- [ ] Codespace 启动无报错
- [ ] Docker 服务自动启动
- [ ] API 可通过 `http://localhost:8888` 访问
- [ ] 数据库返回预期测试数据
- [ ] Java 17 已可用于 Rest Assured 开发

## 风险与缓解

### Potential Issues
1. **端口冲突**
   - *Mitigation*：使用非标准端口，例如 API 用 8888，MySQL 用 3307
2. **Docker 启动时序问题**
   - *Mitigation*：增加健康检查和启动延迟
3. **文件权限问题**
   - *Mitigation*：在 Dockerfile 中设置正确文件权限

### 成功标准
- 学生打开 Codespace 后能立刻开始写测试
- 不需要手工配置或额外初始化
- API 全部可访问且可用
- 数据库已装载真实测试数据
```

</details>

## Step 3：让 goose 生成实施 Prompt

计划已经清楚之后，下一步是让 goose 给出一份足够细致的实施 prompt，便于后续按步骤执行。

### 请求实施 Prompt

**你：** _“这份计划已经很完整了。现在请基于它生成一份详细的 implementation prompt，要求足够具体，让 goose 可以按步骤执行，而不需要额外澄清。”_

### 生成的实施 Prompt

goose 往往会生成类似下面这样的 prompt：

<details>
  <summary>Detailed Prompt</summary>

```markdown
**Task:**
Set up my GitHub repo for GitHub Codespaces with automatic Docker startup for the local API server.

**Instructions:**

1. Create a .devcontainer folder in the root of the repo.
2. Inside .devcontainer, create a devcontainer.json file with the following:
  - Use a Java 17 development image: "image": "mcr.microsoft.com/devcontainers/java:17"
  - Add "features": { "docker-in-docker": "latest" }
  - Set "postCreateCommand": "docker-compose up -d" to automatically start the Docker containers after the Codespace builds.
  - Optionally install VS Code extensions for Java and Docker support ("vscode-java-pack" and "vscode-docker").
3. Create a docker-compose.yml file at the root of the repo.
  - Define the service that runs the API server.
  - Expose necessary ports (e.g., if the API runs on localhost:8888, map it accordingly).

    Example service config:

    `
    version: '3.8'
    services:
      api-server:
        build: ./api-server
        ports:
          - "8888:80"
        volumes:
          - ./api-server:/var/www/html
        depends_on:
          - database (if needed)


  - Adjust the build context and ports based on the actual API being used.

4. If needed, create a Dockerfile under ./api-server/ to define how the API server should be built (especially if this was originally a PHP app in MAMP).
5. Ensure that once a Codespace launches:
  - The API server is already running
  - Users can immediately start writing and running Java tests against the API without any additional setup.
6. Confirm that Docker is working inside the Codespace by using "docker ps" and hitting the API URL in a test after startup.

**Assumptions:**

  - The API server is already included in the repo or will be added.
  - If a database is required (e.g., MySQL for the API), add it as a separate container in docker-compose.yml and configure as needed.

**Deliverables:**

- .devcontainer/devcontainer.json
- docker-compose.yml
- (optional) api-server/Dockerfile
- Confirmed Codespace readiness.
```

</details>

## Step 4：执行计划

现在我们已经有了详细计划和可执行 prompt，就可以按系统化方式开始落实。

### 开始实施

**你：** _“很好，现在按这份计划开始实施。下面是我们整理好的详细 prompt：[贴上 implementation prompt]”_

接下来 goose 会逐项创建所需文件和配置。

### 监控进展

在 goose 执行过程中，你可以用前面那份 checklist 持续对照：

1. **文件是否创建**：确认每个文件都出现在正确位置
2. **配置是否准确**：核对配置是否满足要求
3. **集成点是否打通**：服务间依赖和联通是否正确
4. **是否可测试**：环境是否已经达到可验证状态

### 验证与测试

当实现完成后，按计划中的验证步骤进行检查：

```bash
# 本地测试 Docker 配置
docker-compose up -d
docker ps

# 验证 API 是否可访问
curl http://localhost:8888/api/products

# 测试数据库连接
docker exec -it mysql mysql -u user -p api_db
```

## Step 5：迭代式修正

如果在实施或测试阶段出现问题，就回到计划本身，按阶段定位并逐项修正。

### 常见修正方向

- **端口冲突**：调整映射端口
- **启动时序问题**：增加健康检查或延迟启动
- **权限问题**：在 Dockerfile 中补文件权限设置
- **配置错误**：重新核对环境变量和连接字符串

## 使用 goose 做规划的最佳实践

1. 先脑暴，不要一上来就让它出计划。先通过对话把需求澄清。
2. 明确要求 deliverables。最好直接要求写出文件名、交付物和每步验证方式。
3. 让 goose 识别潜在风险，并给出缓解措施。
4. 让 goose 生成可直接执行的 implementation prompt，减少后续反复澄清。
5. 把验证方式写清楚，这样每一步都有可判断的完成标准。

## 进阶规划技巧

### 多阶段规划

对于非常复杂的项目，可以要求 goose 先把计划拆成多个 phase，再逐步实现：

**你：** _“这个计划挺完整了。你能把它拆成更小的阶段吗？我想每个阶段都能独立实现和验证。”_

### 依赖映射

你还可以让 goose 明确哪些任务能并行、哪些任务有依赖：

**你：** _“这些任务里哪些可以并行做，哪些必须依赖前一步结果？”_

### 替代方案评估

也可以让 goose 帮你比较不同实现路径：

**你：** _“这套实现还有没有更简单、或者更稳健的替代方案？”_

## 总结

goose 的 Plan 功能，能把复杂技术问题从“压力巨大的一团乱麻”，转成“按步骤可执行的路线图”。按本教程的方式，你可以：

1. **Brainstorm**：先梳理需求、补齐隐含前提
2. **Plan**：把复杂任务拆成可执行步骤
3. **Detail**：生成足够细的实施指导
4. **Execute**：按明确的验证标准系统实施
5. **Refine**：根据结果持续修正

你真正需要做的，不是把 goose 当作单纯代码生成器，而是把它当成规划搭档。把完整背景、目标和限制给它，它才能在真正动手之前，先帮你把复杂度理清楚。

## 下一步

- 用这个方法去处理你自己的复杂环境搭建任务
- 试试不同风格的 planning prompt
- 把你的规划实践分享到 [goose community](https://discord.gg/goose-oss)
- 继续探索 planning 如何与 [Lead/Worker mode](/zh-CN/docs/tutorials/lead-worker) 或 [Subagents](/zh-CN/docs/guides/subagents) 配合

真正重要的是：先得到正确的思路、正确的顺序和正确的防护措施。它们决定了结果是“一次性的临时修补”，还是“可维护、可复用的稳健方案”。
