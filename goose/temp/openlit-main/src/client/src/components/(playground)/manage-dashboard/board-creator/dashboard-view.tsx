"use client";

import React, { useCallback } from "react";
import { Responsive, WidthProvider } from "react-grid-layout";
import "react-grid-layout/css/styles.css";
import "react-resizable/css/styles.css";
import { Plus, Download, Settings, ChevronsUpDown, Pencil, Edit, BookText } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { DashboardProps, Widget, WidgetType } from "./types";
import { DashboardProvider, useDashboard } from "./context/dashboard-context";
import WidgetRenderer from "./widgets/widget-renderer";
import dynamic from "next/dynamic";
import createMessage from "@/constants/messages";
const EditWidgetSheet = dynamic(() => import("./components/edit-widget-sheet"));
const WidgetSelectionModal = dynamic(
	() => import("./components/widget-selection-modal")
);

import { useEditWidget } from "./hooks/useEditWidget";

import DescriptionTooltip from "../../../common/description-tooltip";
import UpsertResourceDialog from "../common/upsert-resource-dialog";
import { EditResource, useUpsertResource } from "./hooks/useUpsertResource";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { toast } from "sonner";
import { jsonParse, jsonStringify } from "@/utils/json";
import { Board } from "@/types/manage-dashboard";
import { exportBoardLayout } from "./utils/api";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";

// Empty state component
const EmptyState = ({ onAddWidget }: { onAddWidget: () => void }) => (
	<div className="flex flex-col items-center justify-center h-[70vh] text-center px-4">
		<div className="w-full max-w-md space-y-6">
			<div className="p-6 rounded-lg border-2 border-dashed border-gray-300 dark:border-gray-700">
				<div className="w-12 h-12 rounded-full bg-primary/10 flex items-center justify-center mx-auto mb-4">
					<Plus className="h-6 w-6 text-primary" />
				</div>
				<h3 className="text-lg font-semibold mb-2 text-stone-900 dark:text-stone-300">{createMessage().NO_WIDGETS_YET}</h3>
				<p className="text-sm text-stone-500 dark:text-stone-400 mb-4">
					{createMessage().NO_WIDGETS_YET_DESCRIPTION}
				</p>
				<Button onClick={onAddWidget} className="gap-2">
					<Plus className="h-4 w-4" /> {createMessage().NO_WIDGETS_YET_ACTION_BUTTON}
				</Button>
			</div>
		</div>
	</div>
);

const ActionButtons = ({ onClick, label, className }: { onClick: () => void, label: string, className?: string }) => {
	return (
		<Button variant="secondary" onClick={onClick} className={`flex gap-2 h-8 py-0 text-xs font-normal text-stone-500 dark:text-stone-100 hover:bg-stone-600 dark:hover:bg-stone-600 hover:text-stone-100 border border-stone-200 dark:border-stone-800 ${className ?? ""}`}>
			<span>{label}</span>
		</Button>
	);
};

// Responsive grid layout with automatic width calculation
const ResponsiveGridLayout = WidthProvider(Responsive);

const DashboardContent: React.FC<Omit<DashboardProps, "initialConfig">> = ({
	onSave,
	readonly = false,
	className,
	editorLanguage = "clickhouse-sql",
	breakpoints = { lg: 1200, md: 996, sm: 768, xs: 480, xxs: 0 },
	cols = { lg: 4, md: 4, sm: 2, xs: 1, xxs: 1 },
	rowHeight = 150,
	renderTitle = false,
	runFilters,
	headerComponent,
	handleBoardUpdates,
}) => {
	const {
		details,
		setDetails,
		layouts,
		setLayouts,
		widgets,
		isEditing,
		setIsEditing,
		getDashboardConfig,
		addWidget,
		fetchExistingWidgets,
		removeWidget,
	} = useDashboard();

	const { openEditSheet } = useEditWidget();

	// Modal state
	const [showWidgetModal, setShowWidgetModal] = React.useState(false);
	const [existingWidgets, setExistingWidgets] = React.useState<Widget[]>([]);
	const { fireRequest: updateBoardRequest } = useFetchWrapper();

	const editItem = useCallback(
		({
			itemId,
			newTitle,
			newDescription,
			newTags = [],
		}: EditResource) => {

			// Toast loading state
			toast.loading(`Updating ...`, {
				id: "manage-dashboard-explorer",
			});

			// Create payload
			const payload = {
				id: itemId,
				title: newTitle,
				description: newDescription,
				tags: newTags,
				updatedParent: false,
			};

			updateBoardRequest({
				url: "/api/manage-dashboard/board",
				requestType: "PUT",
				body: jsonStringify(payload),
				successCb: (response) => {
					toast.success("Board updated successfully", {
						id: "manage-dashboard-explorer",
					});
					if (response.data?.[0] satisfies Partial<Board>) {
						setDetails(response.data[0]);
						handleBoardUpdates?.(response.data[0]);
					}
				},
				failureCb: (error) => {
					toast.error(`Failed to update board: ${error || "Unknown error"}`, {
						id: "manage-dashboard-explorer",
					});
				},
			});

			// Close dialog
			setDialogState((prev) => ({
				...prev,
				isOpen: false,
				editingItemId: null,
			}));
		},
		[updateBoardRequest]
	);


	const { dialogState, setDialogState, handleDialogCancel, handleDialogSave } = useUpsertResource({
		editItem,
	});

	// Wrap addWidget to handle modal logic
	const handleAddWidget = async () => {
		if (fetchExistingWidgets) {
			const widgets = await fetchExistingWidgets();
			setExistingWidgets(widgets);
			setShowWidgetModal(true);
		} else {
			addWidget();
		}
	};

	const handleSelectWidget = (widget: Widget) => {
		setShowWidgetModal(false);
		addWidget(widget);
	};

	const handleCreateNew = (widgetType: WidgetType) => {
		setShowWidgetModal(false);
		addWidget({
			type: widgetType,
		});
	};

	// Handle layout changes
	const handleLayoutChange = (layout: any, layouts: any) => {
		setLayouts(layouts);
	};

	// Handle save
	const handleSave = () => {
		if (onSave) {
			onSave(getDashboardConfig());
		}
		setIsEditing(false);
	};

	const openEditDialog = useCallback(
		(item: Partial<Board>) => {
			setDialogState({
				isOpen: true,
				mode: "edit",
				itemTitle: item.title || "",
				itemDescription: item.description || "",
				itemTags: item.tags ? jsonParse(item.tags) : [],
				itemType: "board",
				currentPath: null,
				editingItemId: item.id!,
			});
		},
		[]
	);

	return (
		<>
			{(renderTitle || !readonly || headerComponent) && (
				<div className="flex items-center w-full mb-6 gap-4">
					{renderTitle && (
						<div className="flex items-center gap-2 text-stone-900 dark:text-stone-300">
							<h1 className="text-2xl font-bold">{details.title}</h1>
							{details.description && (
								<DescriptionTooltip description={details.description} className="ml-2 h-4 w-4" icon={<BookText className="text-stone-500 cursor-pointer h-5" />} />
							)}
						</div>
					)}
					{!renderTitle && details.description && (
						<DescriptionTooltip description={details.description} className="ml-2 h-4 w-4" icon={<BookText className="text-stone-500 cursor-pointer h-5" />} />
					)}
					{headerComponent}
					{isEditing && (
						<ActionButtons
							className="bg-primary dark:bg-primary text-stone-100 dark:text-stone-100"
							onClick={() => handleSave()}
							label="Save Layout"
						/>
					)}
					{isEditing && (
						<ActionButtons
							className="text-primary bg-primary/10 dark:bg-primary/10 dark:text-primary"
							onClick={handleAddWidget}
							label={"Create widget"}
						/>
					)}
					{!readonly && (
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<Button variant="outline" className="flex gap-2 shrink-0 justify-start group-data-[state=close]:justify-center overflow-hidden text-stone-500 dark:text-stone-100 hover:bg-stone-600 dark:hover:bg-stone-600 hover:text-stone-100 font-normal py-0 h-8 text-xs">
									<Settings className={`size-3 shrink-0`} />
									<span className="block group-data-[state=close]:hidden text-ellipsis overflow-hidden whitespace-nowrap grow">Actions</span>
									<ChevronsUpDown className={`size-3 block group-data-[state=close]:hidden shrink-0`} />
								</Button>
							</DropdownMenuTrigger>
							<DropdownMenuContent className="w-56" side="bottom" align="end">
								<DropdownMenuLabel>Actions</DropdownMenuLabel>
								<DropdownMenuSeparator />
								<DropdownMenuItem className="p-2 gap-4" onClick={() => openEditDialog({
									id: details.id!,
									title: details.title!,
									description: details.description!,
									tags: details.tags!,
								})}>
									<Pencil className="h-3 w-3" />
									Update Board details
								</DropdownMenuItem>
								{
									isEditing ? null : (
										<DropdownMenuItem className="p-2 gap-4" onClick={() => setIsEditing(true)}>
											<Edit className="h-3 w-3" />
											Edit Layout
										</DropdownMenuItem>
									)
								}
								<DropdownMenuItem className="p-2 gap-4" onClick={() => exportBoardLayout(details.id!)}>
									<Download className="h-3 w-3" />
									Export Layout
								</DropdownMenuItem>
							</DropdownMenuContent>
						</DropdownMenu>

					)}
				</div>
			)}
			<div className={`w-full ${className ?? ""}`}>

				<ResponsiveGridLayout
					className="layout"
					layouts={layouts}
					breakpoints={breakpoints}
					cols={cols}
					rowHeight={rowHeight}
					onLayoutChange={handleLayoutChange}
					isDraggable={isEditing && !readonly}
					isResizable={isEditing && !readonly}
					margin={[16, 16]}
					containerPadding={[0, 0]}
				>
					{
						layouts.lg.map((item: any) => {
							const widget = widgets[item.i];
							if (!widget) return null;

							return (
								<div key={item.i} className="bg-background">
									<WidgetRenderer
										widget={widget}
										isEditing={isEditing && !readonly}
										onEdit={openEditSheet}
										onRemove={(widgetId) => {
											// Implementation of removeWidget is in the DashboardContext
											removeWidget(widgetId);
										}}
										runFilters={runFilters}
									/>
								</div>
							);
						})
					}
				</ResponsiveGridLayout>
				{
					!readonly && (
						<>
							{layouts.lg.length === 0 && (
								<div style={{ width: '100%' }}>
									<EmptyState onAddWidget={() => { setIsEditing(true); handleAddWidget() }} />
								</div>
							)}
							<EditWidgetSheet editorLanguage={editorLanguage} />
							<WidgetSelectionModal
								open={showWidgetModal}
								onClose={() => setShowWidgetModal(false)}
								widgets={existingWidgets}
								onSelect={handleSelectWidget}
								onCreateNew={handleCreateNew}
							/>
							<UpsertResourceDialog
								isOpen={dialogState.isOpen}
								onOpenChange={(open) =>
									setDialogState((prev) => ({ ...prev, isOpen: open }))
								}
								mode={dialogState.mode}
								initialItemTitle={dialogState.itemTitle}
								initialItemDescription={dialogState.itemDescription}
								initialItemType={dialogState.itemType}
								initialItemTags={dialogState.itemTags}
								onSave={handleDialogSave}
								onCancel={handleDialogCancel}
							/>
						</>
					)
				}
			</div>
		</>
	);
};

// Main Dashboard component with provider
const Dashboard: React.FC<DashboardProps> = ({ initialConfig, ...props }) => {
	return (
		<DashboardProvider
			initialConfig={initialConfig}
			onSave={props.onSave}
			runQuery={props.runQuery}
			handleWidgetCrud={props.handleWidgetCrud}
			fetchExistingWidgets={props.fetchExistingWidgets}
		>
			<DashboardContent {...props} />
		</DashboardProvider>
	);
};

export default Dashboard;
