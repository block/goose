"use client";

import { useState } from "react";
import { useDashboard } from "../context/dashboard-context";

export const useEditWidget = () => {
	const {
		editingWidget,
		setEditingWidget,
		widgets,
		updateWidget,
		updateWidgetProperties,
		runQuery,
	} = useDashboard();
	const [currentTab, setCurrentTab] = useState("general");
	const [isFullscreenEditor, setIsFullscreenEditor] = useState(false);

	// Current widget being edited
	const currentWidget = editingWidget ? widgets[editingWidget] : null;

	// Open the edit sheet for a widget
	const openEditSheet = (widgetId: string) => {
		setEditingWidget(widgetId);
		setCurrentTab("general");
	};

	// Close the edit sheet
	const closeEditSheet = () => {
		setEditingWidget(null);
		setIsFullscreenEditor(false);
	};

	// Toggle fullscreen editor
	const toggleFullscreenEditor = () => {
		setIsFullscreenEditor(!isFullscreenEditor);
	};

	return {
		currentTab,
		setCurrentTab,
		isFullscreenEditor,
		currentWidget,
		openEditSheet,
		closeEditSheet,
		toggleFullscreenEditor,
		updateWidget,
		updateWidgetProperties,
		runQuery,
	};
};
