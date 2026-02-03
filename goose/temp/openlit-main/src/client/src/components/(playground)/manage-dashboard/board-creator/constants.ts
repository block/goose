import { ColorTheme, Widget, WidgetType } from "./types"
import { Activity, BarChart3, LineChart, PieChart, Database, AreaChart, FileText, LucideIcon } from "lucide-react"

export const DEFAULT_PRIMARY_COLOR = "#F36C06" as ColorTheme;

export const SELECTOR_COLORS = [
  { name: "Primary", value: DEFAULT_PRIMARY_COLOR },
  { name: "Blue", value: "#0ea5e9" as ColorTheme },
  { name: "Green", value: "#10b981" as ColorTheme },
  { name: "Purple", value: "#8b5cf6" as ColorTheme },
  { name: "Red", value: "#ef4444" as ColorTheme },
  { name: "Yellow", value: "#f59e0b" as ColorTheme },
  { name: "Pink", value: "#ec4899" as ColorTheme },
  { name: "Gray", value: "#6b7280" as ColorTheme },
] as const;

// ClickHouse SQL language configuration for Monaco Editor
export const CLICKHOUSE_LANGUAGE_CONFIG = {
  id: "clickhouse-sql",
  extensions: [".sql"],
  aliases: ["ClickHouse SQL", "clickhouse-sql"],
  mimetypes: ["application/sql"],
  loader: () => ({
    language: {
      defaultToken: "",
      tokenPostfix: ".sql",
      ignoreCase: true,
      brackets: [
        { open: "[", close: "]", token: "delimiter.square" },
        { open: "(", close: ")", token: "delimiter.parenthesis" },
      ],
      keywords: [
        "SELECT",
        "FROM",
        "WHERE",
        "AND",
        "OR",
        "GROUP",
        "BY",
        "ORDER",
        "HAVING",
        "LIMIT",
        "OFFSET",
        "INSERT",
        "INTO",
        "VALUES",
        "UPDATE",
        "DELETE",
        "SET",
        // ... (truncated for brevity)
      ],
      operators: [
        "+",
        "-",
        "*",
        "/",
        "%",
        "=",
        ">",
        "<",
        ">=",
        "<=",
        "<>",
        "!=",
        "<=>",
        "AND",
        "OR",
        "NOT",
        "LIKE",
        "IN",
        "IS",
        "BETWEEN",
        "REGEXP",
      ],
      builtinFunctions: [
        "count",
        "sum",
        "min",
        "max",
        "avg",
        "any",
        "anyHeavy",
        "anyLast",
        // ... (truncated for brevity)
      ],
      builtinVariables: ["database", "table", "default_kind"],
      pseudoColumns: [],
      tokenizer: {
        root: [
          { include: "@comments" },
          { include: "@whitespace" },
          { include: "@numbers" },
          { include: "@strings" },
          { include: "@complexIdentifiers" },
          { include: "@scopes" },
          [/[;,.]/, "delimiter"],
          [/[()]/, "@brackets"],
          [
            /[\w@#$]+/,
            {
              cases: {
                "@keywords": "keyword",
                "@operators": "operator",
                "@builtinFunctions": "predefined",
                "@builtinVariables": "predefined",
                "@default": "identifier",
              },
            },
          ],
          [/[<>=!%&+\-*/|~^]/, "operator"],
        ],
        whitespace: [[/\s+/, "white"]],
        comments: [
          [/--+.*/, "comment"],
          [/\/\*/, { token: "comment.quote", next: "@comment" }],
        ],
        comment: [
          [/[^*/]+/, "comment"],
          [/\*\//, { token: "comment.quote", next: "@pop" }],
          [/./, "comment"],
        ],
        numbers: [
          [/0[xX][0-9a-fA-F]*/, "number"],
          [/[$][+-]*\d*(\.\d*)?/, "number"],
          [/((\d+(\.\d*)?)|(\.\d+))([eE][-+]?\d+)?/, "number"],
        ],
        strings: [
          [/'/, { token: "string", next: "@string" }],
          [/"/, { token: "string.double", next: "@stringDouble" }],
        ],
        string: [
          [/[^']+/, "string"],
          [/''/, "string"],
          [/'/, { token: "string", next: "@pop" }],
        ],
        stringDouble: [
          [/[^"]+/, "string.double"],
          [/""/, "string.double"],
          [/"/, { token: "string.double", next: "@pop" }],
        ],
        complexIdentifiers: [[/`/, { token: "identifier.quote", next: "@quotedIdentifier" }]],
        quotedIdentifier: [
          [/[^`]+/, "identifier"],
          [/``/, "identifier"],
          [/`/, { token: "identifier.quote", next: "@pop" }],
        ],
        scopes: [],
      },
    },
  }),
}

export const SUPPORTED_WIDGETS: Record<WidgetType, {
  name: string;
  description: string;
  icon: LucideIcon;
  initialProperties: Partial<Widget>;
}> = {
  [WidgetType.STAT_CARD]: {
    name: "Stat Card",
    description: "A stat card widget",
    icon: Activity,
    initialProperties: {
      title: "New Stat Card Widget",
      description: "New Widget Description",
      type: WidgetType.STAT_CARD,
      properties: {
        color: DEFAULT_PRIMARY_COLOR,
      },
      config: {},
    }
  },
  [WidgetType.BAR_CHART]: {
    name: "Bar Chart",
    description: "A bar chart widget",
    icon: BarChart3,
    initialProperties: {
      title: "New Bar Chart Widget",
      description: "New Widget Description",
      type: WidgetType.BAR_CHART,
      properties: {
        color: DEFAULT_PRIMARY_COLOR,
      },
      config: {},
    }
  },
  [WidgetType.LINE_CHART]: {
    name: "Line Chart",
    description: "A line chart widget",
    icon: LineChart,
    initialProperties: {
      title: "New Line ChartWidget",
      description: "New Widget Description",
      type: WidgetType.LINE_CHART,
      properties: {
        color: DEFAULT_PRIMARY_COLOR,
      },
      config: {},
    }
  },
  [WidgetType.PIE_CHART]: {
    name: "Pie Chart",
    description: "A pie chart widget",
    icon: PieChart,
    initialProperties: {
      title: "New Pie Chart Widget",
      description: "New Widget Description",
      type: WidgetType.PIE_CHART,
      properties: {
        color: DEFAULT_PRIMARY_COLOR,
      },
      config: {},
    }
  },
  [WidgetType.TABLE]: {
    name: "Table",
    description: "A table widget",
    icon: Database,
    initialProperties: {
      title: "New Table Widget",
      description: "New Widget Description",
      type: WidgetType.TABLE,
      properties: {
        color: DEFAULT_PRIMARY_COLOR,
      },
      config: {},
    }
  },
  [WidgetType.AREA_CHART]: {
    name: "Area Chart",
    description: "An area chart widget",
    icon: AreaChart,
    initialProperties: {
      title: "New Area Chart Widget",
      description: "New Widget Description",
      type: WidgetType.AREA_CHART,
      properties: {},
      config: {},
    }
  },
  [WidgetType.MARKDOWN]: {
    name: "Markdown",
    description: "A markdown widget",
    icon: FileText,
    initialProperties: {
      title: "New Markdown Widget",
      description: "New Widget Description",
      type: WidgetType.MARKDOWN,
      properties: {
        color: DEFAULT_PRIMARY_COLOR,
      },
      config: {},
    }
  }
}