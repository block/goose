---
sidebar_position: 25
title: "更新 goose"
sidebar_label: "更新 goose"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { DesktopAutoUpdateSteps } from '@site/src/components/DesktopAutoUpdateSteps';
import MacDesktopInstallButtons from '@site/src/components/MacDesktopInstallButtons';
import WindowsDesktopInstallButtons from '@site/src/components/WindowsDesktopInstallButtons';
import LinuxDesktopInstallButtons from '@site/src/components/LinuxDesktopInstallButtons';

goose CLI 和 Desktop 应用都处于持续开发中。为了获得最新功能、修复和性能改进，建议你定期按下面的方法更新 goose 客户端。

<Tabs>
  <TabItem value="mac" label="macOS" default>
    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        把 goose 更新到最新稳定版。

        <DesktopAutoUpdateSteps />
        
        **如果要手动下载安装更新：**
        1. <MacDesktopInstallButtons/>
        2. 解压下载得到的 zip 文件
        3. 将解压出的 `Goose.app` 拖到 `Applications` 文件夹中，覆盖当前版本
        4. 启动 goose Desktop

      </TabItem>
      <TabItem value="cli" label="goose CLI">
        你可以运行下面的命令更新 goose：

        ```sh
        goose update
        ```

        其他可用[选项](/zh-CN/docs/guides/goose-cli-commands#update-options)：
        
        ```sh
        # Update to latest canary (development) version
        goose update --canary

        # Update and reconfigure settings
        goose update --reconfigure
        ```

        你也可以重新执行[安装脚本](../getting-started/installation.md)：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        如果要查看当前 goose 版本，请运行：

        ```sh
        goose --version
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="linux" label="Linux">
    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        把 goose 更新到最新稳定版。

        <DesktopAutoUpdateSteps />
        
        **如果要手动下载安装更新：**
        1. <LinuxDesktopInstallButtons/>

        #### 对 Debian / Ubuntu 系发行版
        2. 在终端中切换到下载好的 DEB 文件所在目录
        3. 运行 `sudo dpkg -i (filename).deb`
        4. 从应用菜单中启动 goose
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        你可以运行下面的命令更新 goose：

        ```sh
        goose update
        ```

        其他可用[选项](/zh-CN/docs/guides/goose-cli-commands#update-options)：
        
        ```sh
        # Update to latest canary (development) version
        goose update --canary

        # Update and reconfigure settings
        goose update --reconfigure
        ```

        你也可以重新执行[安装脚本](../getting-started/installation.md)：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        如果要查看当前 goose 版本，请运行：

        ```sh
        goose --version
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="windows" label="Windows">
    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        把 goose 更新到最新稳定版。

        <DesktopAutoUpdateSteps />
        
        **如果要手动下载安装更新：**
        1. <WindowsDesktopInstallButtons/>
        2. 解压下载得到的 zip 文件
        3. 运行解压出的可执行文件来启动 goose Desktop
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        你可以运行下面的命令更新 goose：

        ```sh
        goose update
        ```

        其他可用[选项](/zh-CN/docs/guides/goose-cli-commands#update-options)：
        
        ```sh
        # Update to latest canary (development) version
        goose update --canary

        # Update and reconfigure settings
        goose update --reconfigure
        ```

        你也可以在 **Git Bash**、**MSYS2** 或 **PowerShell** 中重新执行[安装脚本](../getting-started/installation.md)，在 Windows 原生环境下更新 goose CLI：

        ```bash
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```
        
        如果要查看当前 goose 版本，请运行：

        ```sh
        goose --version
        ```        

        <details>
        <summary>通过 Windows Subsystem for Linux (WSL) 更新</summary>

        如果你是在 WSL 中安装的 goose，可以使用 `goose update`，或通过 WSL 重新执行安装脚本：

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

       </details>
      </TabItem>
    </Tabs>
  </TabItem>
</Tabs>

:::info 在 CI/CD 中更新
如果你是在 CI 或其他非交互环境中运行 goose，建议通过 `GOOSE_VERSION` 固定版本，以获得可复现的安装结果。完整示例和使用方式请参考 [CI/CD Environments](/zh-CN/docs/tutorials/cicd)。
:::
