import React, { useMemo, memo, useEffect } from "react";
import { Edit, Trash } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { WidgetType, type WidgetRendererProps } from "../types";
import { SUPPORTED_WIDGETS } from "../constants";
import StatCardWidget from "./stat-card-widget";
import BarChartWidget from "./bar-chart-widget";
import LineChartWidget from "./line-chart-widget";
import PieChartWidget from "./pie-chart-widget";
import TableWidget from "./table-widget";
import AreaChartWidget from "./area-chart-widget";
import MarkdownWidget from "./markdown-widget";
import { useDashboard } from "../context/dashboard-context";
import DescriptionTooltip from "../../../../common/description-tooltip";

const WidgetRenderer: React.FC<WidgetRendererProps> = ({
	widget,
	isEditing,
	onEdit,
	onRemove,
	runFilters,
}) => {
	const { widgetData, loadWidgetData } = useDashboard();

	// Load data when not in edit mode
	useEffect(() => {
		// @ts-ignore TODO: fix this
		if (widget?.config?.query) {
			loadWidgetData(widget.id);
		}
	}, [runFilters]);

	// Get widget type icon
	const WidgetTypeIcon = () => {
		const IconComponent =
			SUPPORTED_WIDGETS[widget.type as keyof typeof SUPPORTED_WIDGETS]?.icon;
		return <IconComponent className="h-4 w-4" />;
	};

	const widgetEvaluatedData = widgetData[widget.id];

	const WidgetComponent = useMemo(() => {
		switch (widget.type) {
			case WidgetType.STAT_CARD:
				return StatCardWidget;
			case WidgetType.BAR_CHART:
				return BarChartWidget;
			case WidgetType.LINE_CHART:
				return LineChartWidget;
			case WidgetType.PIE_CHART:
				return PieChartWidget;
			case WidgetType.TABLE:
				return TableWidget;
			case WidgetType.AREA_CHART:
				return AreaChartWidget;
			case WidgetType.MARKDOWN:
				return MarkdownWidget;
			default:
				return null;
		}
	}, [widget]);

	return (
		<Card className="h-full flex flex-col hover:shadow-lg shadow-stone-200 dark:shadow-stone-800 border border-stone-200 dark:border-stone-800" data-widget-id={widget.id}>
			<CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
				<div className="flex items-center gap-2">
					<WidgetTypeIcon />
					<CardTitle className="tracking-tight text-sm font-medium text-stone-600 dark:text-stone-300">{widget.title}</CardTitle>
					{widget.description && (
						<DescriptionTooltip description={widget.description} />
					)}
				</div>
				{isEditing && (
					<div
						className="flex gap-1 z-10"
						// This makes the div and its children non-draggable
						onMouseDown={(e) => {
							e.stopPropagation();
						}}
						onTouchStart={(e) => {
							e.stopPropagation();
						}}
					>
						<div
							className="cursor-pointer p-1 rounded-md hover:bg-muted"
							onClick={() => onEdit(widget.id)}
						>
							<Edit className="h-4 w-4" />
						</div>
						<div
							className="cursor-pointer p-1 rounded-md hover:bg-muted"
							onClick={() => onRemove(widget.id)}
						>
							<Trash className="h-4 w-4" />
						</div>
					</div>
				)}
			</CardHeader>
			<CardContent className="flex-grow overflow-auto">
				{WidgetComponent && (
					<WidgetComponent data={widgetEvaluatedData} widget={widget as any} />
				)}
			</CardContent>
		</Card>
	);
};

export default memo(WidgetRenderer);
