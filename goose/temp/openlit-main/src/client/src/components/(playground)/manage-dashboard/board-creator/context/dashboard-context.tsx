"use client";

import type React from "react";
import { createContext, useContext, useState, type ReactNode } from "react";
import {
	type WidgetsRecord,
	type DashboardConfig,
	type Widget,
	WidgetType,
	StatCardWidget,
	TableWidget,
	PieChartWidget,
	LineChartWidget,
	BarChartWidget,
} from "../types";
import { SUPPORTED_WIDGETS } from "../constants";
import { Board } from "@/types/manage-dashboard";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";

interface DashboardContextType {
	details: Partial<Board>;
	setDetails: (details: Partial<Board>) => void;
	layouts: any;
	setLayouts: (layouts: any) => void;
	widgets: WidgetsRecord;
	setWidgets: (widgets: WidgetsRecord) => void;
	editingWidget: string | null;
	setEditingWidget: (id: string | null) => void;
	isEditing: boolean;
	setIsEditing: (editing: boolean) => void;
	updateWidget: (widgetId: string, updates: Partial<Widget>) => void;
	updateWidgetProperties: (
		widgetId: string,
		properties: Record<string, any>
	) => void;
	addWidget: (widget?: Partial<Widget>) => Promise<any>;
	removeWidget: (widgetId: string) => void;
	getDashboardConfig: () => DashboardConfig;
	runQuery: (
		widgetId: string,
		params: Record<string, unknown>
	) => Promise<{ data: any; err: string | null }>;
	handleWidgetCrud?: (updates: Partial<Widget>) => Promise<Widget>;
	widgetData: Record<string, any>;
	updateWidgetData: (widgetId: string, data: any) => void;
	clearWidgetData: (widgetId: string) => void;
	loadWidgetData: (widgetId: string) => Promise<void>;
	fetchExistingWidgets?: () => Promise<Widget[]>;
}

export const DashboardContext = createContext<DashboardContextType | undefined>(
	undefined
);

interface DashboardProviderProps {
	children: ReactNode;
	initialConfig?: DashboardConfig;
	onSave?: (config: DashboardConfig) => void;
	handleWidgetCrud?: (updates: Partial<Widget>) => Promise<Widget>;
	runQuery?: (
		widgetId: string,
		params: Record<string, unknown>
	) => Promise<{ data: any; err: string | null }>;
	fetchExistingWidgets?: () => Promise<Widget[]>;
}

export const DashboardProvider: React.FC<DashboardProviderProps> = ({
	children,
	initialConfig,
	onSave,
	handleWidgetCrud,
	runQuery,
	fetchExistingWidgets,
}) => {
	const [details, setDetails] = useState<Partial<Board>>({
		id: initialConfig?.id || "",
		title: initialConfig?.title || "",
		description: initialConfig?.description || "",
		tags: initialConfig?.tags || "",
	});
	const [layouts, setLayouts] = useState(initialConfig?.layouts || { lg: [] });
	const [widgets, setWidgets] = useState<WidgetsRecord>(
		initialConfig?.widgets || {}
	);
	const [widgetData, setWidgetData] = useState<Record<string, any>>({});
	const [editingWidget, setEditingWidget] = useState<string | null>(null);
	const [isEditing, setIsEditing] = useState(false);
	const posthog = usePostHog();

	// Load widget data
	const loadWidgetData = async (widgetId: string) => {
		const widget = widgets[widgetId] as StatCardWidget | BarChartWidget | LineChartWidget | PieChartWidget | TableWidget;
		if (widget.type === WidgetType.MARKDOWN || !widget?.config?.query) return;

		try {
			if (runQuery) {
			const { data } = await runQuery(widgetId, {});
			updateWidgetData(widgetId, data);
		}
	} catch (error) {
		// Use separate parameters to prevent log injection
		console.error('Failed to load data for widget:', widgetId, error);
	}
	};

	// Update widget data
	const updateWidgetData = (widgetId: string, data: any) => {
		setWidgetData((prev) => ({
			...prev,
			[widgetId]: data,
		}));
	};

	// Clear widget data
	const clearWidgetData = (widgetId: string) => {
		setWidgetData((prev) => {
			const newData = { ...prev };
			delete newData[widgetId];
			return newData;
		});
	};

	// Update a widget
	const updateWidget = (widgetId: string, updates: Partial<Widget>) => {
		setWidgets((prev) => ({
			...prev,
			[widgetId]: {
				...prev[widgetId],
				...updates,
			} as Widget,
		}));
	};

	// Update widget properties
	const updateWidgetProperties = (
		widgetId: string,
		properties: Record<string, any>
	) => {
		setWidgets((prev) => {
			const widget = prev[widgetId];
			if (!widget) return prev;

			const updatedWidget = {
				...widget,
				properties: {
					...(widget.properties || {}),
					...properties,
				},
			} as Widget;

			return {
				...prev,
				[widgetId]: updatedWidget,
			};
		});
	};

	// Add a new widget
	const addWidget = async (existingWidget?: Partial<Widget>) => {
		const newWidgetId = `widget-${Object.keys(widgets).length + 1
			}-${Date.now()}`;

		let newWidget: Partial<Widget>;

		if (existingWidget?.id) {
			newWidget = existingWidget;
		} else {
			try {
				newWidget =
					existingWidget?.type ?
						SUPPORTED_WIDGETS[existingWidget.type as WidgetType].initialProperties :
						SUPPORTED_WIDGETS[WidgetType.STAT_CARD].initialProperties;
				if (handleWidgetCrud) {
					newWidget = await handleWidgetCrud(newWidget);
				} else {
					newWidget.id = newWidgetId;
				}

				posthog?.capture(CLIENT_EVENTS.DASHBOARD_WIDGET_ADDED, {
					id: newWidget.id,
					type: newWidget.type,
					title: newWidget.title,
				});
			} catch (error) {
				posthog?.capture(CLIENT_EVENTS.DASHBOARD_WIDGET_ADD_FAILURE, {
					type: existingWidget?.type,
					title: existingWidget?.title,
					error: error?.toString(),
				});
				return;
			}
		}

		if (newWidget.id) {
			// Add to layouts
			setLayouts((prev: any) => {
				return {
					...prev,
					lg: [
						...prev.lg,
						{ i: newWidget.id, x: 0, y: Number.POSITIVE_INFINITY, w: 2, h: 2 },
					],
				};
			});

			// Add widget data with defaults
			setWidgets((prev) => ({
				...prev,
				[newWidget.id!]: newWidget as Widget,
			}));

			// Set this widget as the editing widget and enable editing mode
			setEditingWidget(newWidgetId);
			setIsEditing(true);

			// Scroll the widget into view after a short delay to ensure the widget is rendered
			setTimeout(() => {
				const widgetElement = document.querySelector(
					`[data-widget-id="${newWidget.id}"]`
				);
				if (widgetElement) {
					widgetElement.scrollIntoView({ behavior: "smooth", block: "center" });
				}
				setEditingWidget(newWidget.id!);
			}, 100);

			return newWidgetId;
		}
	};

	// Remove a widget
	const removeWidget = (widgetId: string) => {
		// Remove from layouts
		setLayouts((prev: any) => {
			return {
				...prev,
				lg: prev.lg.filter((item: any) => item.i !== widgetId),
			};
		});

		// Remove widget data
		setWidgets((prev) => {
			const newWidgets = { ...prev };
			delete newWidgets[widgetId];
			return newWidgets;
		});

		// Clear widget data
		clearWidgetData(widgetId);
	};

	// Get the current dashboard configuration
	const getDashboardConfig = (): DashboardConfig => {
		return {
			id: initialConfig?.id || "",
			title: initialConfig?.title || "",
			description: initialConfig?.description || "",
			tags: initialConfig?.tags || "",
			layouts,
			widgets,
		};
	};

	const handleRunQuery = (widgetId: string, params: Record<string, unknown>) => {
		if (runQuery) {
			posthog?.capture(CLIENT_EVENTS.DASHBOARD_WIDGET_RUN_QUERY, {
				id: widgetId,
				type: widgets[widgetId]?.type,
				title: widgets[widgetId]?.title,
			});
			return runQuery(widgetId, params);
		}

		return Promise.resolve({ data: [], err: null });
	};

	const contextValue: DashboardContextType = {
		details,
		setDetails,
		layouts,
		setLayouts,
		widgets,
		setWidgets,
		editingWidget,
		setEditingWidget,
		isEditing,
		setIsEditing,
		updateWidget,
		updateWidgetProperties,
		addWidget,
		removeWidget,
		getDashboardConfig,
		runQuery: handleRunQuery,
		handleWidgetCrud,
		widgetData,
		updateWidgetData,
		clearWidgetData,
		loadWidgetData,
		fetchExistingWidgets,
	};

	return (
		<DashboardContext.Provider value={contextValue}>
			{children}
		</DashboardContext.Provider>
	);
};

export const useDashboard = () => {
	const context = useContext(DashboardContext);
	if (!context) {
		throw new Error("useDashboard must be used within a DashboardProvider");
	}
	return context;
};
