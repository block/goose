---
title: "Recipes 教程"
description: "介绍如何创建和使用 goose recipes，并理解 prompt、参数与 MCP server 的组织方式。"
---

# Recipes 教程

goose recipes 是把某一类任务完整描述清楚的文件。因为所有信息都收在一个文件里，所以它非常适合通过日常文件共享方式传播，也适合放进 git 这类版本管理系统。我们先从一个最简单的 recipe 开始。

## 最简单的 Recipe

最简单的 recipe，本质上就是一个 prompt。看起来这似乎不算太有用，毕竟 prompt 发 Slack 或邮件也一样能共享；但现实里，用户之所以没法让 agent 做对事情，最常见的原因恰恰是 prompt 太短，而且没有持续迭代。把 prompt 存进文本文件，会同时改善这两个问题。

下面是一个帮用户规划欧洲旅行的 recipe：

```yaml
title: Trip planner
description: Plan your next trip
prompt: |
 Help the user plan a trip to Europe for 14 days.
 Create a detailed itinerary that includes:
  - places to visit
  - activities to do
  - local cuisine to try
  - a rough budget estimate
```

你可以在命令行里这样运行它：

```sh
goose run --recipe trip.yaml
```

## Extensions

recipe 里有一个专门的部分，用来声明执行期间 goose 可以使用哪些[扩展](/zh-CN/docs/guides/recipes/recipe-reference)。goose 只会使用你显式列出来的扩展。

假设我们希望旅行期间天气尽量好，那么就可以给 recipe 增加一个天气扩展（下面这个例子使用的是 TuanKiri 基于 MIT License 发布的 [weather-mcp-server](https://github.com/TuanKiri/weather-mcp-server)），再稍微修改一下 prompt，这样 goose 就会在把某个城市放进行程前先检查天气。

```yaml
title: Trip planner
description: Plan your next trip
prompt: |
 Help the user plan a trip to Europe for 14 days. Create a detailed itinerary that includes:
  - places to visit
  - activities to do
  - local cuisine to try
  - a rough budget estimate
 Ensure that the user has good weather throughout their trip. Optimize their trip based on the forecast in potential locations.
extensions:
  - type: stdio
    name: weathermcpserver
    cmd: /Users/svega/Development/weather-mcp-server/weather-mcp-server
    args: []
    timeout: 300
    description: "Weather data for trip planning"
    env_keys:
      - WEATHER_API_KEY
```

## Parameters

你也可以通过 parameters 让 recipe 变成动态模板。参数就是在运行 recipe 时由用户提供的变量。每个参数都有数据类型，也有一个 requirement 字段，用来说明它是必填、可选，还是由系统提供。比如，我们可以给这个旅行 recipe 增加旅行目的地和天数两个参数：

```yaml
parameters:
  - key: destination
    input_type: string
    requirement: required
    description: Destination for the trip. Should be a large region with multiple climates.
  - key: duration
    input_type: number
    requirement: required
    description: Number of days for the trip.
```

recipe 使用模板系统，因此你可以在 prompt 里写 `{{ destination }}` 这样的变量占位符，运行时会被实际值替换。更新完 prompt 之后，你就可以这样运行它，为一个 14 天的非洲之旅生成计划：

```sh
goose run --recipe trip.yaml --params destination=Africa --params duration=14
```

## Settings

默认情况下，goose 会沿用你全局配置里的 `temperature` 和 `model`，大多数时候这已经足够。但有些任务你可能希望更细粒度控制。例如旅行规划这类更主观的任务，适当提高 `temperature` 往往更有帮助。可以把它理解成一个创造力旋钮：值越高，结果越多样、越出人意料。如果第一次输出不理想，用户也可以直接重新运行 recipe，得到另一版建议。

你还可以为某个 recipe 指定专属的 AI provider 和 model：

```yaml
settings:
  goose_provider: "anthropic"
  goose_model: "claude-sonnet-4-20250514"
  temperature: 0.8
```

可用设置包括：

- `goose_provider`：AI provider，例如 `"anthropic"`、`"openai"`
- `goose_model`：具体模型名称
- `temperature`：控制创造力/随机性（`0.0-1.0`，越高越有创造性）

这些设置会在 recipe 运行时覆盖你的默认 goose 配置。

## 外部文件

有时候，你会想给 agent 更多信息，但又不想把所有数据都塞进 prompt 里。此时，更好的做法是把额外信息存进单独文件，再让 recipe 指向这个文件。

为此，recipes 内置了一个变量 `{{ recipe_dir }}`，可以让你引用和 recipe 放在一起的文件。比如，你可以从 [Kaggle](https://www.kaggle.com/datasets/ramjasmaurya/unesco-heritage-sites2021?resource=download) 下载 UNESCO 遗产名录，并在旅行规划 recipe 中使用它。

然后你可以在 prompt 里这样引用：

```yaml
prompt: |
 You can use the \{\{ recipe_dir \}\}/unesco.csv file to
 check information on UNESCO world heritage sites to
 include in your travel plan.
```

同时你还要声明一个能够读取文件的扩展：

```yaml
extensions:
 - type: builtin
   name: developer
   display_name: Developer
   timeout: 300
   bundled: true
```

这里我们加的是 [Developer 扩展](/zh-CN/docs/mcp/developer-mcp)，它允许 agent 读取文件，从中提取相关信息。

:::info Example Recipe Output

<details>
<summary>查看详细的 10 天欧洲行程示例</summary>

基于 UNESCO 世界遗产信息和当前天气预报，下面是一份详细的 10 天欧洲行程：

# 10 天欧洲冒险之旅行程

这份行程会带你走过法国、意大利和捷克三个兼具文化与风景魅力的国家。你会体验世界级博物馆、UNESCO 世界遗产、美食，以及丰富的本地文化。

#### 第 1-3 天：法国巴黎 🇫🇷

**第 1 天：抵达巴黎**
- **上午**：抵达戴高乐机场，前往酒店
- **下午**：沿塞纳河轻松散步，参观巴黎圣母院外观（因修复暂无法完全开放）
- **晚上**：在拉丁区晚餐（预算：€30-40）
  - 推荐尝试经典法式洋葱汤和红酒炖鸡

**天气预报**：气温约 27°C，局部多云，体感舒适

**第 2 天：巴黎精华**
- **上午**：参观卢浮宫（预算：€17）
- **下午**：游览杜乐丽花园和香榭丽舍大街
- **晚上**：傍晚前往埃菲尔铁塔看日落（预算：登顶 €26.80）
  - 在特罗卡德罗附近晚餐（预算：€35-45）
  - 推荐尝试蜗牛和勃艮第炖牛肉

**天气预报**：31°C，晴朗

**第 3 天：凡尔赛一日游**
- **上午**：前往凡尔赛宫（UNESCO 世界遗产）（预算：宫殿门票 €21）
- **下午**：游览宏伟花园
- **晚上**：返回巴黎，在蒙马特晚餐（预算：€30-40）
  - 推荐尝试可丽饼和油封鸭

**天气预报**：30°C，略有降雨概率

#### 第 4-6 天：意大利罗马 🇮🇹

**第 4 天：前往罗马**
- **上午**：从巴黎飞往罗马（预算：€100-150）
- **下午**：办理入住，游览西班牙台阶和特莱维喷泉
- **晚上**：在特拉斯提弗列区晚餐（预算：€25-35）
  - 推荐尝试 cacio e pepe 和 carbonara

**天气预报**：35°C，晴朗炎热

**第 5 天：古罗马**
- **上午**：参观斗兽场和古罗马广场（联票预算：€16）
- **下午**：游览帕拉蒂尼山和大竞技场遗址
- **晚上**：在鲜花广场附近晚餐（预算：€30-40）
  - 推荐尝试罗马披萨和 saltimbocca alla romana

**天气预报**：35°C，大部晴天

**第 6 天：梵蒂冈**
- **上午**：参观梵蒂冈博物馆和西斯廷礼拜堂（预算：€17）
- **下午**：圣彼得大教堂和圣彼得广场（UNESCO 世界遗产）
- **晚上**：在 Prati 区晚餐（预算：€30-40）
  - 推荐尝试 suppli（炸饭团）和 osso buco

**天气预报**：34°C，局部多云

#### 第 7-10 天：捷克布拉格 🇨🇿

**第 7 天：前往布拉格**
- **上午**：从罗马飞往布拉格（预算：€100-150）
- **下午**：入住酒店，游览老城广场
- **晚上**：在老城区晚餐（预算：€20-30）
  - 推荐尝试传统炖牛肉和 svíčková

**天气预报**：29°C，局部多云，较舒适

**第 8 天：布拉格城堡与小城区**
- **上午**：参观布拉格城堡建筑群（UNESCO 世界遗产）（预算：250 CZK / €10）
- **下午**：游览小城区和查理大桥
- **晚上**：在伏尔塔瓦河景餐厅晚餐（预算：€25-35）
  - 推荐尝试 trdelník 和烤鸭配饺子

**天气预报**：22°C，可能有雷暴

**第 9 天：犹太区与文化地标**
- **上午**：参观犹太区（Josefov）及其会堂
- **下午**：市民会馆和火药塔
- **晚上**：体验捷克民俗晚宴和表演（预算：€35-45）
  - 推荐尝试捷克啤酒和 kolache 甜点

**天气预报**：24°C，大部晴天

**第 10 天：返程**
- **上午**：在布拉格精品街区最后购物
- **下午**：前往机场返程

**天气预报**：24°C，舒适晴朗

#### 预算拆解（每人）

**住宿（9 晚）**
- 巴黎：€120/晚 × 3 晚 = €360
- 罗马：€100/晚 × 3 晚 = €300
- 布拉格：€80/晚 × 3 晚 = €240
- **住宿合计**：€900

**交通**
- 往返欧洲国际机票：€600-800（视出发地而定）
- 巴黎到罗马：€100-150
- 罗马到布拉格：€100-150
- 市内交通（地铁/公交/电车）：€15/天 × 10 天 = €150
- **交通合计**：€950-1,250

**景点和活动**
- 巴黎博物馆和景点：€100
- 罗马博物馆和景点：€80
- 布拉格博物馆和景点：€70
- **景点合计**：€250

**餐饮**
- 早餐：€10/天 × 10 天 = €100
- 午餐：€15/天 × 10 天 = €150
- 晚餐：€35/天 × 10 天 = €350
- 零食和饮品：€10/天 × 10 天 = €100
- **餐饮合计**：€700

**其他**
- 旅行保险：€50
- 纪念品和购物：€200
- 预备金：€150
- **其他合计**：€400

**总预算**
- **每人 €3,200-3,500**（不含往返欧洲的国际机票）

#### 涵盖的 UNESCO 世界遗产
- 凡尔赛宫及其花园（法国）
- 罗马历史中心（意大利）
- 梵蒂冈城（意大利）
- 布拉格历史中心（捷克）

#### 旅行建议
1. **天气**：按当前预报，三地都偏暖，温度范围约 20-35°C。布拉格晚上偏凉，建议带一件薄外套。
2. **货币**：法国和意大利用欧元（€），捷克使用捷克克朗（CZK）。
3. **交通**：建议在各城市购买公共交通通票以节省费用。
4. **预约**：卢浮宫、梵蒂冈博物馆和埃菲尔铁塔这类热门景点，建议提前预约。
5. **饮水**：尤其在罗马高温天气下，建议随身带可重复装水的水瓶。
6. **语言**：可以提前学几句基础当地语言，但多数旅游区域都能使用英语。

这份行程在历史、文化和美食之间做了很好的平衡，也覆盖了欧洲不同区域的城市体验。天气条件整体非常适合观光，大部分时间会是温暖且晴朗的天气。祝你欧洲之旅顺利！

</details>

:::

## 继续阅读

查看 [Recipes 指南](/zh-CN/docs/guides/recipes)，获取更多文档、工具和资源，进一步掌握 goose recipes。
