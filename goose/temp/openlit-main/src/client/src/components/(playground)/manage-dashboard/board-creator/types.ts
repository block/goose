import type { ReactNode } from "react";
import type { Layouts } from "react-grid-layout";
import { Board } from "@/types/manage-dashboard";

// Widget Types
export enum WidgetType {
	STAT_CARD = "STAT_CARD",
	BAR_CHART = "BAR_CHART",
	LINE_CHART = "LINE_CHART",
	PIE_CHART = "PIE_CHART",
	TABLE = "TABLE",
	AREA_CHART = "AREA_CHART",
	MARKDOWN = "MARKDOWN",
}

type RGB = `rgb(${number}, ${number}, ${number})`;
type RGBA = `rgba(${number}, ${number}, ${number}, ${number})`;
type HEX = `#${string}`;

// Color Themes
export type ColorTheme = RGB | RGBA | HEX;

// Base Widget Interface
export interface BaseWidgetProps {
	id: string;
	title: string;
	type: WidgetType;
	description?: string;
	config?: Record<string, any>;
	properties: Record<string, any>;
}

// Specific Widget Interfaces
export interface StatCardWidget extends BaseWidgetProps {
	properties: {
		prefix?: string;
		suffix?: string;
		value?: string;
		color?: ColorTheme;
		trend?: string;
		trendSuffix?: string;
		trendPrefix?: string;
		textSize?: "small" | "medium" | "large";
		autoRefresh?: boolean;
	};
}

export interface ChartWidget extends BaseWidgetProps {
	properties: {
		color: ColorTheme;
		showLegend?: boolean;
		autoRefresh?: boolean;
	};
}

export interface BarChartWidget extends ChartWidget {
	properties: ChartWidget["properties"] & {
		xAxis: string;
		yAxis: string;
	};
}

export interface LineChartWidget extends ChartWidget {
	properties: ChartWidget["properties"] & {
		xAxis: string;
		yAxis: string;
	};
}

export interface PieChartWidget extends ChartWidget {
	properties: ChartWidget["properties"] & {
		labelPath: string;
		valuePath: string;
	};
}

export interface TableWidget extends BaseWidgetProps {
	properties: {
		color: ColorTheme;
		autoRefresh?: boolean;
	};
}

export interface AreaChartWidget extends ChartWidget {
	properties: ChartWidget["properties"] & {
		xAxis: string;
		yAxes: {
			key: string;
			color: ColorTheme;
		}[];
		stackId?: string;
	};
}

export interface MarkdownWidget extends BaseWidgetProps {
	config: {
		content: string;
		showPreview?: boolean;
	};
	properties: {
		color: ColorTheme;
	};
}

// Combined Widget Type
export type Widget =
	| StatCardWidget
	| BarChartWidget
	| LineChartWidget
	| PieChartWidget
	| TableWidget
	| AreaChartWidget
	| MarkdownWidget;

// Widgets Record - maps widget IDs to widget objects
export type WidgetsRecord = Record<string, Widget>;

// Dashboard Configuration
export interface DashboardConfig {
	id: string;
	title: string;
	description: string;
	tags?: string;
	layouts: Layouts;
	widgets: WidgetsRecord;
}

// Dashboard Props
export interface DashboardProps {
	initialConfig?: DashboardConfig;
	onSave?: (config: DashboardConfig) => void;
	readonly?: boolean;
	className?: string;
	renderCustomWidget?: (widget: Widget) => ReactNode;
	editorLanguage?: string;
	customTheme?: any;
	breakpoints?: { [key: string]: number };
	cols?: { [key: string]: number };
	rowHeight?: number;
	runQuery?: (
		widgetId: string,
		params: Record<string, unknown>
	) => Promise<{ data: any; err: string | null }>;
	handleWidgetCrud?: (updates: Partial<Widget>) => Promise<Widget>;
	fetchExistingWidgets?: () => Promise<Widget[]>;
	renderTitle?: boolean;
	runFilters?: unknown;
	headerComponent?: ReactNode;
	handleBoardUpdates?: (details: Partial<Board>) => void;
}

// Widget Renderer Props
export interface WidgetRendererProps {
	widget: Widget;
	isEditing: boolean;
	onEdit: (widgetId: string) => void;
	onRemove: (widgetId: string) => void;
	runFilters?: unknown;
}

// Editor Props
export interface EditorProps {
	value: string;
	onChange: (value: string | undefined) => void;
	language?: string;
	height?: string;
	fullScreen?: boolean;
	readOnly?: boolean;
}
