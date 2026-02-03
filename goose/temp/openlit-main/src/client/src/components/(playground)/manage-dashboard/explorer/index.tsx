import { Droppable, DroppableProps } from "react-beautiful-dnd";
import { DashboardItemType } from "@/types/manage-dashboard";
import { DashboardHeirarchy } from "@/types/manage-dashboard";
import { DropResult } from "react-beautiful-dnd";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { useCallback, useEffect, useState } from "react";
import { DragDropContext } from "react-beautiful-dnd";
import { toast } from "sonner";
import EmptyState from "../common/empty-state";
import ExplorerItemRow from "./item-row";
import UpsertResourceDialog from "../common/upsert-resource-dialog";
import { jsonParse, jsonStringify } from "@/utils/json";
import { useRouter } from "next/navigation";
import RootActions from "./root-actions";
import Header from "../common/header";
import getMessage from "@/constants/messages";
import { Button } from "@/components/ui/button";
import { usePageHeader } from "@/selectors/page";
import { AddResource, EditResource, useUpsertResource } from "../board-creator/hooks/useUpsertResource";
import { exportBoardLayout } from "../board-creator/utils/api";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import { PRIMARY_BACKGROUND } from "@/constants/common-classes";

// React 18 StrictMode compatibility fix for react-beautiful-dnd
const StrictModeDroppable = ({ children, ...props }: DroppableProps) => {
	const [enabled, setEnabled] = useState(false);
	
	useEffect(() => {
		const animation = requestAnimationFrame(() => setEnabled(true));
		return () => {
			cancelAnimationFrame(animation);
			setEnabled(false);
		};
	}, []);
	
	if (!enabled) {
		return null;
	}
	
	return <Droppable {...props}>{children}</Droppable>;
};

export default function DashboardExplorer() {
	const [items, setItems] = useState<DashboardHeirarchy[]>([]);
	const [isLoading, setIsLoading] = useState(true);

	const { fireRequest: fetchHierarchy } = useFetchWrapper();
	const { fireRequest: createFolderRequest } = useFetchWrapper();
	const { fireRequest: createBoardRequest } = useFetchWrapper();
	const { fireRequest: updateFolderRequest } = useFetchWrapper();
	const { fireRequest: updateBoardRequest } = useFetchWrapper();
	const { fireRequest: deleteFolderRequest } = useFetchWrapper();
	const { fireRequest: deleteBoardRequest } = useFetchWrapper();
	const { fireRequest: setMainDashboardRequest } = useFetchWrapper();
	const { fireRequest: importBoardLayoutRequest } = useFetchWrapper();
	const { setHeader } = usePageHeader();
	const posthog = usePostHog();

	const router = useRouter();
	// Fetch hierarchy data on component mount
	useEffect(() => {
		loadHierarchy();
	}, []);

	// Load hierarchy data from API
	const loadHierarchy = useCallback(() => {
		setIsLoading(true);
		fetchHierarchy({
			url: "/api/manage-dashboard/folder/get-heirarchy",
			requestType: "GET",
			successCb: (response) => {
				if (response?.data) {
					setItems(response.data);
					setHeader({
						title: "Dashboards",
						breadcrumbs: [],
					});
				}
				setIsLoading(false);
				posthog?.capture(CLIENT_EVENTS.DASHBOARD_EXPLORER_LOADED, {
					count: response?.data?.length || 0,
				});
			},
			failureCb: (error) => {
				toast.error("Failed to load dashboard hierarchy");
				posthog?.capture(CLIENT_EVENTS.DASHBOARD_EXPLORER_LOAD_FAILURE, {
					error: error?.toString(),
				});
				setIsLoading(false);
			},
		});
	}, [fetchHierarchy]);

	// Add a new item
	const addItem = useCallback(
		({
			title,
			description,
			type,
			tags = [],
			parentPath = [],
		}: AddResource) => {

			// Create payload
			const payload = {
				title,
				description,
				parentId: parentPath && parentPath.length > 0 ? parentPath[parentPath.length - 1] : null,
				tags: tags,
			};

			// Toast loading state
			toast.loading(`Creating ${type}...`, { id: "manage-dashboard-explorer" });

			if (type === "folder") {
				createFolderRequest({
					url: "/api/manage-dashboard/folder",
					requestType: "POST",
					body: jsonStringify(payload),
					successCb: (response) => {
						toast.success("Folder created successfully", {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_FOLDER_CREATED, {
							id: response.data.id,
							title: response.data.title,
						});
						loadHierarchy(); // Reload hierarchy after successful creation
					},
					failureCb: (error) => {
						toast.error(
							`Failed to create folder: ${error || "Unknown error"}`,
							{ id: "manage-dashboard-explorer" }
						);
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_FOLDER_CREATE_FAILURE, {
							error: error?.toString(),
						});
					},
				});
			} else {
				createBoardRequest({
					url: "/api/manage-dashboard/board",
					requestType: "POST",
					body: jsonStringify(payload),
					successCb: (response) => {
						toast.success("Board created successfully", {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_CREATED, {
							id: response.data.id,
							title: response.data.title,
						});
						router.push(`/d/${response.data.id}`);
					},
					failureCb: (error) => {
						toast.error(`Failed to create board: ${error || "Unknown error"}`, {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_CREATE_FAILURE, {
							error: error?.toString(),
						});
					},
				});
			}

			// Close dialog
			setDialogState((prev) => ({
				...prev,
				isOpen: false,
			}));
		},
		[createFolderRequest, createBoardRequest, loadHierarchy]
	);

	// Edit an item
	const editItem = useCallback(
		({
			itemId,
			newTitle,
			newDescription,
			newTags = [],
			parentPath = [],
		}: EditResource) => {
			// Find the item to determine its type
			const findItem = (
				items: DashboardHeirarchy[],
				id: string
			): DashboardHeirarchy | null => {
				for (const item of items) {
					if (item.id === id) return item;
					if (item.children) {
						const found = findItem(item.children, id);
						if (found) return found;
					}
				}
				return null;
			};

			const item = findItem(items, itemId);
			if (!item) {
				toast.error("Item not found", { id: "manage-dashboard-explorer" });
				return;
			}

			// Toast loading state
			toast.loading(`Updating ${item.type}...`, {
				id: "manage-dashboard-explorer",
			});

			// Create payload
			const payload = {
				id: itemId,
				title: newTitle,
				description: newDescription,
				parentId: parentPath && parentPath.length > 0 ? parentPath[parentPath.length - 1] : null,
				tags: newTags,
				updatedParent: false,
			};

			if (item.type === "folder") {
				updateFolderRequest({
					url: "/api/manage-dashboard/folder",
					requestType: "PUT",
					body: jsonStringify(payload),
					successCb: (response) => {
						toast.success("Folder updated successfully", {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_FOLDER_UPDATED, {
							id: response.data.id,
							title: response.data.title,
						});
						loadHierarchy(); // Reload hierarchy after successful update
					},
					failureCb: (error) => {
						toast.error(
							`Failed to update folder: ${error || "Unknown error"}`,
							{ id: "manage-dashboard-explorer" }
						);
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_FOLDER_UPDATE_FAILURE, {
							error: error?.toString(),
						});
					},
				});
			} else {
				updateBoardRequest({
					url: "/api/manage-dashboard/board",
					requestType: "PUT",
					body: jsonStringify(payload),
					successCb: (response) => {
						toast.success("Board updated successfully", {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_UPDATED, {
							id: response.data.id,
							title: response.data.title,
						});
						loadHierarchy(); // Reload hierarchy after successful update
					},
					failureCb: (error) => {
						toast.error(`Failed to update board: ${error || "Unknown error"}`, {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_UPDATE_FAILURE, {
							error: error?.toString(),
						});
					},
				});
			}

			// Close dialog
			setDialogState((prev) => ({
				...prev,
				isOpen: false,
				editingItemId: null,
			}));
		},
		[items, updateFolderRequest, updateBoardRequest, loadHierarchy]
	);

	const { dialogState, setDialogState, handleDialogSave, handleDialogCancel } = useUpsertResource({
		addItem,
		editItem,
	});

	// Delete an item
	const deleteItem = useCallback(
		(itemId: string) => {
			// Find the item to determine its type
			const findItem = (
				items: DashboardHeirarchy[],
				id: string
			): DashboardHeirarchy | null => {
				for (const item of items) {
					if (item.id === id) return item;
					if (item.children) {
						const found = findItem(item.children, id);
						if (found) return found;
					}
				}
				return null;
			};

			const item = findItem(items, itemId);
			if (!item) {
				toast.error("Item not found", { id: "manage-dashboard-explorer" });
				return;
			}

			// Confirm deletion
			if (
				!window.confirm(
					`Are you sure you want to delete this ${item.type}? This action cannot be undone.`
				)
			) {
				return;
			}

			// Toast loading state
			toast.loading(`Deleting ${item.type}...`, {
				id: "manage-dashboard-explorer",
			});

			if (item.type === "folder") {
				deleteFolderRequest({
					url: `/api/manage-dashboard/folder/${itemId}`,
					requestType: "DELETE",
					successCb: (response) => {
						toast.success("Folder deleted successfully", {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_FOLDER_DELETED, {
							id: response.data.id,
							title: response.data.title,
						});
						loadHierarchy(); // Reload hierarchy after successful deletion
					},
					failureCb: (error) => {
						toast.error(
							`Failed to delete folder: ${error || "Unknown error"}`,
							{ id: "manage-dashboard-explorer" }
						);
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_FOLDER_DELETE_FAILURE, {
							error: error?.toString(),
						});
					},
				});
			} else {
				deleteBoardRequest({
					url: `/api/manage-dashboard/board/${itemId}`,
					requestType: "DELETE",
					successCb: (response) => {
						toast.success("Board deleted successfully", {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_DELETED, {
							id: response.data.id,
							title: response.data.title,
						});
						loadHierarchy(); // Reload hierarchy after successful deletion
					},
					failureCb: (error) => {
						toast.error(`Failed to delete board: ${error || "Unknown error"}`, {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_DELETE_FAILURE, {
							error: error?.toString(),
						});
					},
				});
			}
		},
		[items, deleteFolderRequest, deleteBoardRequest, loadHierarchy]
	);

	const handleSetMainDashboard = useCallback(
		(boardId: string) => {
			setMainDashboardRequest({
				requestType: "PATCH",
				url: `/api/manage-dashboard/board/${boardId}`,
				body: jsonStringify({ setMain: true }),
				successCb: () => {
					posthog?.capture(CLIENT_EVENTS.DASHBOARD_SET_MAIN_DASHBOARD, {
						id: boardId,
					});
					loadHierarchy();
				},
				failureCb: (error) => {
					toast.error("Failed to set as main dashboard", {
						id: "manage-dashboard-explorer",
					});
					posthog?.capture(CLIENT_EVENTS.DASHBOARD_SET_MAIN_DASHBOARD_FAILURE, {
						error: error?.toString(),
					});
				},
			});
		},
		[loadHierarchy, setMainDashboardRequest]
	);

	const handleUpdatePinnedBoard = useCallback(
		(boardId: string) => {
			setMainDashboardRequest({
				requestType: "PATCH",
				url: `/api/manage-dashboard/board/${boardId}`,
				body: jsonStringify({ updatePinned: true }),
				successCb: () => {
					posthog?.capture(CLIENT_EVENTS.DASHBOARD_TOGGLE_PINNED, {
						id: boardId,
					});
					loadHierarchy();
				},
				failureCb: (error) => {
					toast.error("Failed to update pinned board", {
						id: "manage-dashboard-explorer",
					});
					posthog?.capture(CLIENT_EVENTS.DASHBOARD_TOGGLE_PINNED_FAILURE, {
						error: error?.toString(),
					});
				},
			});
		},
		[loadHierarchy, setMainDashboardRequest]
	);

	const importBoardLayout = useCallback((data: any) => {
		return importBoardLayoutRequest({
			requestType: "POST",
			url: `/api/manage-dashboard/board/layout/import`,
			body: jsonStringify(data),
			successCb: (response) => {
				posthog?.capture(CLIENT_EVENTS.DASHBOARD_IMPORT_SUCCESS, {
					id: response.data.id,
				});
				router.push(`/d/${response.data.id}`);
			},
			failureCb: (error) => {
				toast.error("Failed to import board layout", {
					id: "manage-dashboard-explorer",
				});
				posthog?.capture(CLIENT_EVENTS.DASHBOARD_IMPORT_FAILURE, {
					error: error?.toString(),
				});
			},
		});
	}, [importBoardLayoutRequest, loadHierarchy]);

	// Open dialog for adding a new item
	const openAddDialog = useCallback((path: string[] = []) => {
		setDialogState({
			isOpen: true,
			mode: "add",
			itemTitle: "",
			itemDescription: "",
			itemTags: [],
			itemType: "board",
			currentPath: path,
			editingItemId: null,
		});
	}, []);

	// Open dialog for editing an item
	const openEditDialog = useCallback(
		(item: DashboardHeirarchy, path: string[] = []) => {
			setDialogState({
				isOpen: true,
				mode: "edit",
				itemTitle: item.title,
				itemDescription: item.description,
				itemTags: item.tags ? jsonParse(item.tags) : [],
				itemType: item.type,
				currentPath: path,
				editingItemId: item.id,
			});
		},
		[]
	);

	// Navigate to a board
	const navigateToBoard = useCallback((boardId: string) => {
		router.push(`/d/${boardId}`);
	}, []);

	// Handle drag end
	const handleDragEnd = useCallback(
		(result: DropResult) => {
			const { source, destination, draggableId } = result;

			// Dropped outside the list
			if (!destination) return;

			// Dropped in the same position
			if (
				source.droppableId === destination.droppableId &&
				source.index === destination.index
			) {
				return;
			}

			// Update UI immediately for better UX
			setItems((prevItems) => {
				const clonedItems = JSON.parse(jsonStringify(prevItems));

				// Helper function to find an item and its parent array by ID
				const findItemAndParent = (
					items: DashboardHeirarchy[],
					itemId: string,
					path: string[] = []
				): {
					item: DashboardHeirarchy | null;
					parent: DashboardHeirarchy[] | null;
					itemPath: string[];
				} => {
					for (let i = 0; i < items.length; i++) {
						if (items[i].id === itemId) {
							return { item: items[i], parent: items, itemPath: path };
						}

						if (items[i].type === "folder" && items[i].children) {
							const result = findItemAndParent(
								items[i].children || [],
								itemId,
								[...path, items[i].id]
							);
							if (result.item) {
								return result;
							}
						}
					}

					return { item: null, parent: null, itemPath: [] };
				};

				// Helper function to find a parent folder by droppable ID
				const findParentByDroppableId = (
					items: DashboardHeirarchy[],
					droppableId: string
				): { parent: DashboardHeirarchy[] | null; parentId: string | null } => {
					if (droppableId === "root") {
						return { parent: clonedItems, parentId: null };
					}

					// Extract folder ID from droppable ID (format: "folder-id")
					const folderId = droppableId.replace("folder-", "");

					const { item } = findItemAndParent(items, folderId);
					return {
						parent: item && item.children ? item.children : null,
						parentId: folderId,
					};
				};

				// Find the item being dragged and its parent array
				const { item: draggedItem, parent: sourceParent } = findItemAndParent(
					clonedItems,
					draggableId
				);

				if (!draggedItem || !sourceParent) return clonedItems;

				// Remove the item from its source
				sourceParent.splice(sourceParent.indexOf(draggedItem), 1);

				// Find the destination parent array
				const { parent: destinationParent, parentId: newParentId } =
					findParentByDroppableId(clonedItems, destination.droppableId);

				if (!destinationParent) return clonedItems;

				// Update the parentId of the dragged item
				draggedItem.parentId = newParentId;

				// Insert the item at the destination
				destinationParent.splice(destination.index, 0, draggedItem);

				return clonedItems;
			});

			// Send update to backend
			const getNewParentId = (droppableId: string) => {
				if (droppableId === "root") return null;
				return droppableId.replace("folder-", "");
			};

			// Find the item to determine its type
			const findItem = (
				items: DashboardHeirarchy[],
				id: string
			): DashboardHeirarchy | null => {
				for (const item of items) {
					if (item.id === id) return item;
					if (item.children) {
						const found = findItem(item.children, id);
						if (found) return found;
					}
				}
				return null;
			};

			const item = findItem(items, draggableId);
			if (!item) return;

			const newParentId = getNewParentId(destination.droppableId);

			// Create payload
			const payload = {
				id: draggableId,
				title: item.title,
				description: item.description,
				parentId: newParentId,
				updateParent: true,
			};

			// Toast loading state
			toast.loading(`Updating ${item.type}...`, {
				id: "manage-dashboard-explorer",
			});

			if (item.type === "folder") {
				updateFolderRequest({
					url: "/api/manage-dashboard/folder",
					requestType: "PUT",
					body: jsonStringify(payload),
					successCb: (response) => {
						toast.success("Folder moved successfully", {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_FOLDER_UPDATED, {
							id: response.data.id,
							title: response.data.title,
							reorder: true,
						});
						loadHierarchy(); // Reload hierarchy to ensure consistency
					},
					failureCb: (error) => {
						toast.error(`Failed to move folder: ${error || "Unknown error"}`, {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_FOLDER_UPDATE_FAILURE, {
							error: error?.toString(),
							reorder: true,
						});
						loadHierarchy(); // Reload hierarchy to reset UI
					},
				});
			} else {
				updateBoardRequest({
					url: "/api/manage-dashboard/board",
					requestType: "PUT",
					body: jsonStringify(payload),
					successCb: (response) => {
						toast.success("Board moved successfully", {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_UPDATED, {
							id: response.data.id,
							title: response.data.title,
							reorder: true,
						});
						loadHierarchy(); // Reload hierarchy to ensure consistency
					},
					failureCb: (error) => {
						toast.error(`Failed to move board: ${error || "Unknown error"}`, {
							id: "manage-dashboard-explorer",
						});
						posthog?.capture(CLIENT_EVENTS.DASHBOARD_UPDATE_FAILURE, {
							error: error?.toString(),
							reorder: true,
						});
						loadHierarchy(); // Reload hierarchy to reset UI
					},
				});
			}
		},
		[items, updateFolderRequest, updateBoardRequest, loadHierarchy]
	);


	return (
		<div className="flex flex-col gap-2 grow overflow-y-hidden">
			<Header title="Explorer">
				<RootActions openAddDialog={openAddDialog} importBoardLayout={importBoardLayout} />
			</Header>
			<div className={`grow ${PRIMARY_BACKGROUND} border border-stone-200 dark:border-stone-800 rounded-sm p-2 overflow-y-auto`}>
				{isLoading ? (
					<div className="flex justify-center items-center py-8 h-full">
						<div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
					</div>
				) : items.length === 0 ? (
					<EmptyState
						title={getMessage().NO_DASHBOARDS_YET}
						description={getMessage().NO_DASHBOARDS_YET_DESCRIPTION}
					>
						<Button onClick={() => openAddDialog()}>{getMessage().NO_DASHBOARDS_YET_ACTION_BUTTON}</Button>
					</EmptyState>
				) : (
					<DragDropContext onDragEnd={handleDragEnd}>
						<StrictModeDroppable droppableId="root" type="explorer-item">
							{(provided, snapshot) => (
								<div
									ref={provided.innerRef}
									{...provided.droppableProps}
									className={`${snapshot.isDraggingOver
										? "bg-stone-200 dark:bg-stone-800 border-2 border-dashed border-stone-300 dark:border-stone-600 rounded-md"
										: ""
										}`}
								>
									{items.map((item, index) => (
										<ExplorerItemRow
											key={item.id}
											item={item}
											path={[]}
											index={index}
											onNavigate={navigateToBoard}
											onAddClick={openAddDialog}
											onEditClick={openEditDialog}
											onDeleteClick={deleteItem}
											exportBoardLayout={exportBoardLayout}
											setMainDashboard={handleSetMainDashboard}
											updatePinnedBoard={handleUpdatePinnedBoard}
											importBoardLayout={importBoardLayout}
										/>
									))}
									{provided.placeholder}
								</div>
							)}
						</StrictModeDroppable>
					</DragDropContext>
				)}
			</div>

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
		</div>
	);
}
