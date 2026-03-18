---
title: "构建自定义扩展"
description: "说明如何创建自定义 MCP Server 并作为 goose 扩展使用。"
---

import { PanelLeft } from 'lucide-react';
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# 构建自定义扩展

goose 允许你通过自定义扩展扩展能力，而这些扩展本质上都是 MCP server。之所以能够兼容 goose，是因为 goose 遵循 [Model Context Protocol (MCP)][mcp-docs]。MCP 是一个开放协议，用来标准化应用向 LLM 提供上下文的方式，让 LLM 可以用一致的方法连接不同数据源和工具，因此非常适合用来构建结构化、可互操作的扩展。

本教程会使用 [MCP 的 Python SDK][mcp-python] 构建一个 MCP server：读取 Wikipedia 文章并把它转换为 Markdown，然后将它作为扩展接入 goose。你也可以用同样的流程构建属于自己的 goose 扩展。

你还可以参考 [MCP servers 仓库][mcp-servers] 中的其他示例。MCP 也提供了其他常见语言的 SDK，例如 [TypeScript][mcp-typescript] 和 [Kotlin][mcp-kotlin]。

:::info
goose 支持 [Model Context Protocol](https://modelcontextprotocol.io/) 里的 Tools、Resources 和 Prompts。当前支持的协议版本和 client capabilities 可以参考 [`mcp_client.rs`](https://github.com/block/goose/blob/main/crates/goose/src/agents/mcp_client.rs)。
:::

---

## 前置条件

开始之前，请确保系统中已经安装：

- **Python 3.13 或更高版本**：用于运行 MCP server
- **[uv](https://docs.astral.sh/uv/)**：本教程用来管理 Python 项目和依赖
- **Node.js 和 npm**：只有在你准备使用 [Step 4](#step-4-test-your-mcp-server) 中的 MCP Inspector 时才需要

---

## Step 1：初始化项目

第一步是使用 [uv][uv-docs] 创建一个新项目。这里我们把项目命名为 `mcp-wiki`。

在终端中运行以下命令，先搭出 MCP server 的基础目录结构：

```bash
uv init --lib mcp-wiki
cd mcp-wiki

mkdir -p src/mcp_wiki
touch src/mcp_wiki/server.py
touch src/mcp_wiki/__main__.py
```

项目目录结构大致如下：

```plaintext
.
├── README.md
├── pyproject.toml
└── src
    └── mcp_wiki
        ├── __init__.py   # 主 CLI 入口
        ├── __main__.py   # 允许作为 Python module 运行
        ├── py.typed      # 声明包支持类型提示
        └── server.py     # MCP server 代码（tool、resources、prompts）
```

---

## Step 2：编写 MCP Server 代码

这一步我们来实现 MCP server 的核心逻辑，主要由几个文件组成：

1. **`server.py`**：主 MCP server 代码。这个示例中我们会定义一个读取 Wikipedia 文章的 tool。你也可以在这里继续添加自己的 tools、resources 和 prompts。
2. **`__init__.py`**：MCP server 的主 CLI 入口。
3. **`__main__.py`**：让 MCP server 能以 Python module 形式运行。

下面是这个 Wikipedia MCP server 的完整示例实现。

### `server.py`

```python
import requests
from requests.exceptions import RequestException
from bs4 import BeautifulSoup
from html2text import html2text
from urllib.parse import urlparse

from mcp.server.fastmcp import FastMCP
from mcp.shared.exceptions import McpError
from mcp.types import ErrorData, INTERNAL_ERROR, INVALID_PARAMS

mcp = FastMCP("wiki")

@mcp.tool()
def read_wikipedia_article(url: str) -> str:
    """
    Fetch a Wikipedia article at the provided URL, parse its main content,
    convert it to Markdown, and return the resulting text.

    Usage:
        read_wikipedia_article("https://en.wikipedia.org/wiki/Python_(programming_language)")
    """
    try:
        # Validate input
        if not url.startswith("http"):
            raise ValueError("URL must start with http or https.")

        # SSRF protection: only allow Wikipedia domains
        parsed = urlparse(url)
        hostname = parsed.netloc.lower()

        # Allow wikipedia.org or *.wikipedia.org subdomains only
        if hostname != 'wikipedia.org' and not hostname.endswith('.wikipedia.org'):
            raise ValueError(f"Only Wikipedia URLs are allowed. Got: {parsed.netloc}")

        # Add User-Agent header to avoid 403 from Wikipedia
        headers = {
            'User-Agent': 'MCP-Wiki/1.0 (Educational purposes; Python requests)'
        }
        response = requests.get(url, headers=headers, timeout=10)
        if response.status_code != 200:
            raise McpError(
                ErrorData(
                    code=INTERNAL_ERROR,
                    message=f"Failed to retrieve the article. HTTP status code: {response.status_code}"
                )
            )

        soup = BeautifulSoup(response.text, "html.parser")
        content_div = soup.find("div", {"id": "mw-content-text"})
        if not content_div:
            raise McpError(
                ErrorData(
                    code=INVALID_PARAMS,
                    message="Could not find the main content on the provided Wikipedia URL."
                )
            )

        # Convert to Markdown
        markdown_text = html2text(str(content_div))
        return markdown_text

    except ValueError as e:
        raise McpError(ErrorData(code=INVALID_PARAMS, message=str(e))) from e
    except RequestException as e:
        raise McpError(ErrorData(code=INTERNAL_ERROR, message=f"Request error: {str(e)}")) from e
    except Exception as e:
        raise McpError(ErrorData(code=INTERNAL_ERROR, message=f"Unexpected error: {str(e)}")) from e
```

### `__init__.py`

```python
import argparse
from .server import mcp

def main():
    """MCP Wiki: Read Wikipedia articles and convert them to Markdown."""
    parser = argparse.ArgumentParser(
        description="Gives you the ability to read Wikipedia articles and convert them to Markdown."
    )
    parser.parse_args()
    mcp.run()

if __name__ == "__main__":
    main()
```

### `__main__.py`

```python
from mcp_wiki import main

main()
```

---

## Step 3：定义项目配置

接下来在 `pyproject.toml` 中定义项目配置。下面这个配置会声明 CLI script，让 `mcp-wiki` 命令可以直接作为可执行命令使用：

```toml
[project]
name = "mcp-wiki"
version = "0.1.0"
description = "MCP Server for Wikipedia"
readme = "README.md"
requires-python = ">=3.13"
dependencies = [
    "beautifulsoup4>=4.14.0",
    "html2text>=2025.4.15",
    "mcp[cli]>=1.25.0",
    "requests>=2.32.3",
]

[project.scripts]
mcp-wiki = "mcp_wiki:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
```

---

## Step 4：测试你的 MCP Server {#step-4-test-your-mcp-server}

你可以在 MCP Inspector（浏览器里的开发工具）或者 server CLI 中验证 MCP server 是否工作正常。

<Tabs>
  <TabItem value="ui" label="在 MCP Inspector 中" default>
:::info
MCP Inspector 需要你的电脑上安装 Node.js 和 npm。
:::

1. 初始化项目环境：

   ```bash
   uv sync
   ```

2. 激活虚拟环境：

   ```bash
   source .venv/bin/activate
   ```

3. 以开发模式运行 server：

   ```bash
   mcp dev src/mcp_wiki/server.py
   ```

   首次运行时，MCP Inspector 会自动在浏览器中打开，并提示安装 `@modelcontextprotocol/inspector`。

4. 测试工具：
   1. 点击 `Connect` 初始化 MCP server
   2. 在 `Tools` 标签中点击 `List Tools`，再点开 `read_wikipedia_article`
   3. 输入 `https://en.wikipedia.org/wiki/Bangladesh` 作为 URL，并点击 `Run Tool`

[![MCP Inspector UI](../../docs/assets/guides/custom-extension-mcp-inspector.png)](../../docs/assets/guides/custom-extension-mcp-inspector.png)

  </TabItem>
  <TabItem value="cli" label="在 CLI 中">
1. 初始化项目环境：

   ```bash
   uv sync
   ```

2. 激活虚拟环境：

   ```bash
   source .venv/bin/activate
   ```

3. 在本地安装项目：

   ```bash
   uv pip install .
   ```

4. 验证 CLI：

   ```bash
   mcp-wiki --help
   ```

   你应该看到类似这样的输出：

   ```plaintext
   ❯ mcp-wiki --help
   usage: mcp-wiki [-h]

   Gives you the ability to read Wikipedia articles and convert them to Markdown.

   options:
     -h, --help  show this help message and exit
   ```
  </TabItem>
</Tabs>

---

## Step 5：接入 goose

要把你的 MCP server 作为扩展接入 goose，可以按下面的步骤操作：

1. 先构建扩展二进制入口：

   ```bash
   uv pip install .
   ```

2. 打开 goose Desktop，点击左上角的 <PanelLeft className="inline" size={16} /> 按钮打开侧边栏
3. 在侧边栏里点击 `Extensions`
4. 将 `Type` 设置为 `STDIO`
5. 为扩展填写名称和描述
6. 在 `Command` 字段里填入可执行文件的绝对路径：

   ```plaintext
   uv run /full/path/to/mcp-wiki/.venv/bin/mcp-wiki
   ```

   例如：

   ```plaintext
   uv run /Users/smohammed/Development/mcp/mcp-wiki/.venv/bin/mcp-wiki
   ```

:::tip 代码变更后记得重新构建
如果你在接入 goose 后又修改了 MCP server 代码，需要重新执行 `uv pip install .`，然后重启 goose Desktop，改动才会生效。
:::

为了便于本地开发，本教程默认直接运行本地版本。另一种做法是把包发布到 PyPI；发布后，你也可以直接通过 `uvx` 启动：

```bash
uvx mcp-wiki
```

---

## Step 6：在 goose 中使用你的扩展

接入完成后，你就可以在 goose 里直接使用这个扩展。打开 goose 的聊天界面，按需要调用你的工具即可。

如果想确认 goose 是否已经识别到你的自定义扩展工具，可以直接问它：

> what tools do you have?

![goose Chat - Ask about tools](../../docs/assets/guides/custom-extension-tools.png)

然后你就可以尝试提出需要使用该扩展的问题。

![goose Chat - Use custom extension](../../docs/assets/guides/custom-extension-chat.png)

🎉 **恭喜！** 你已经成功构建并把一个自定义 MCP server 接入到了 goose。

---

## MCP 扩展的高级能力

goose 还支持一些 MCP 的高级能力，可以进一步增强你的扩展。

### MCP Sampling：带 AI 能力的工具

**[MCP Sampling](/zh-CN/docs/guides/mcp-sampling)** 允许你的 MCP server 向 goose 当前使用的 LLM 请求 AI completion，从而把“普通工具”提升为“带智能的 agent”。

**主要收益：**

- MCP server 自身不需要再单独持有 OpenAI/Anthropic API key
- 工具可以分析数据、生成解释并做更智能的判断
- 用户体验更好，响应也更具上下文感知
- 默认隔离且自动归因，安全性更高

**如何开始：**

- 在 MCP server 中使用 `sampling/createMessage` 方法请求 AI 能力
- [goose 的实现](https://github.com/block/goose/blob/main/crates/goose/src/agents/mcp_client.rs) 当前支持文本和图片两种内容类型
- goose 会自动向所有 MCP server 宣告 sampling capability

**使用场景：** 文档摘要、智能搜索过滤、代码分析、数据洞察

**更多信息：** 技术细节可参考 [MCP Specification](https://modelcontextprotocol.io/specification/draft/client/sampling)。

### MCP Apps：交互式扩展

**[MCP Apps](/zh-CN/docs/tutorials/building-mcp-apps)** 允许扩展返回丰富的交互式界面，而不只是纯文本响应。

**主要收益：**

- 你的 MCP server 工具可以直接返回交互式 UI 组件
- 这些组件会在 goose Desktop 的隔离 sandbox 中安全渲染
- 用户的实时交互会通过回调重新触发 server 逻辑

**使用场景：** 交互式表单、数据可视化、预订界面、配置向导

**更多信息：** 见 [Building MCP Apps 教程](/zh-CN/docs/tutorials/building-mcp-apps)

:::note
goose 同时也支持 [MCP-UI](/zh-CN/docs/guides/interactive-chat/mcp-ui)，但对于新扩展，官方更推荐直接走 MCP Apps。
:::

[mcp-docs]: https://modelcontextprotocol.io/
[mcp-python]: https://github.com/modelcontextprotocol/python-sdk
[mcp-typescript]: https://github.com/modelcontextprotocol/typescript-sdk
[mcp-kotlin]: https://github.com/modelcontextprotocol/kotlin-sdk
[mcp-servers]: https://github.com/modelcontextprotocol/servers
[uv-docs]: https://docs.astral.sh/uv/getting-started/
