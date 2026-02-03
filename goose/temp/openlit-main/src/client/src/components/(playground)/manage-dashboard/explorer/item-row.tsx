import { DashboardHeirarchy } from "@/types/manage-dashboard";
import { GripVertical, ChevronRight, ChevronDown, PinIcon, HomeIcon } from "lucide-react";
import { useCallback, useState } from "react";
import { Draggable, Droppable } from "react-beautiful-dnd";
import ItemIcon from "./item-icon";
import ItemActions from "./item-actions";
import { Badge } from "@/components/ui/badge";
import { jsonParse } from "@/utils/json";
import { useDashboardPageSearch } from "@/selectors/dashboards";

export default function ExplorerItemRow({
	item,
	path,
	index,
	onNavigate,
	onAddClick,
	onEditClick,
	onDeleteClick,
	exportBoardLayout,
	setMainDashboard,
	updatePinnedBoard,
	importBoardLayout,
}: {
	item: DashboardHeirarchy;
	path: string[];
	index: number;
	onNavigate: (id: string) => void;
	onAddClick: (path: string[]) => void;
	onEditClick: (item: DashboardHeirarchy, path: string[]) => void;
	onDeleteClick: (id: string, path: string[]) => void;
	exportBoardLayout: (id: string) => void;
	setMainDashboard: (id: string) => void;
	updatePinnedBoard: (id: string) => void;
	importBoardLayout: (data: any) => Promise<unknown>;
}) {
	const [open, setOpen] = useState(false);
	const pageSearch = useDashboardPageSearch();

	const handleItemClick = useCallback(() => {
		if (item.type === "board") {
			onNavigate(item.id);
		}
	}, [item.id, item.type, onNavigate]);

	const handleToggleFolder = useCallback((e: React.MouseEvent) => {
		e.stopPropagation();
		setOpen((prev) => !prev);
	}, []);

	const handleImportBoardLayout = useCallback((data: any) => {
		data.parentId = item.id;
		return importBoardLayout(data);
	}, [importBoardLayout]);

	const tags = item.tags ? jsonParse(item.tags) : [];

	const isSearchMatchTitle = item.title.toLowerCase().includes(pageSearch.toLowerCase());
	const isSearchMatchTags = typeof tags === "string" ? tags.toLowerCase().includes(pageSearch.toLowerCase()) : tags?.some?.((tag: string) => tag.toLowerCase().includes(pageSearch.toLowerCase()));
	const isSearchMatch = isSearchMatchTitle || isSearchMatchTags;

	if (!isSearchMatch && item.type !== "folder") {
		return null;
	}

	return (
		<Draggable draggableId={item.id} index={index}>
			{(provided, snapshot) => (
				<div
					ref={provided.innerRef}
					{...provided.draggableProps}
					className="my-2"
				>
					<div className={`${snapshot.isDragging ? "opacity-50" : ""}`}>
						<div className="flex items-center justify-between group py-1 px-2 rounded-sm  hover:bg-primary/10 dark:hover:bg-primary/10 group-hover  bg-transparent dark:bg-transparent">
							<div className="flex items-center gap-2 flex-1">
								<div
									{...provided.dragHandleProps}
									className="cursor-grab group-hover:opacity-100 dark:group-hover:opacity-100 opacity-20"
									onClick={(e) => e.stopPropagation()}
								>
									<GripVertical className="h-4 w-4 text-stone-500 dark:text-stone-400" />
								</div>
								{item.type === "folder" && (
									<button
										onClick={handleToggleFolder}
										className="flex items-center justify-center p-0.5 rounded hover:bg-stone-200 dark:hover:bg-stone-600 transition-colors"
										aria-label={open ? "Collapse folder" : "Expand folder"}
									>
										{open ? (
											<ChevronDown className="h-4 w-4 text-stone-500 dark:text-stone-400" />
										) : (
											<ChevronRight className="h-4 w-4 text-stone-500 dark:text-stone-400" />
										)}
									</button>
								)}
								<div
									className="flex items-center gap-2 cursor-pointer flex-1 text-stone-800 dark:text-stone-200"
									onClick={handleItemClick}
								>
									<ItemIcon
										type={item.type}
										open={item.type === "folder" ? open : undefined}
									/>
									<span>{item.title}</span>
									{item.type === "board" && item.isMainDashboard && (
										<Badge className="text-xs" variant="secondary" title="Main Dashboard">
											<HomeIcon className="h-3 w-3" />
										</Badge>
									)}
									{item.type === "board" && item.isPinned && (
										<Badge className="text-xs" variant="secondary">
											<PinIcon className="h-3 w-3" />
										</Badge>
									)}
									{tags && tags.length > 0 && (
										<div className="flex items-center gap-2 ml-4">
											{tags?.map?.((tag: string) => (
												<Badge key={tag}>{tag}</Badge>
											))}
										</div>
									)}
								</div>
							</div>

							<ItemActions
								item={item}
								path={path}
								onAddClick={onAddClick}
								onEditClick={onEditClick}
								onDeleteClick={onDeleteClick}
								exportBoardLayout={exportBoardLayout}
								setMainDashboard={setMainDashboard}
								updatePinnedBoard={updatePinnedBoard}
								importBoardLayout={handleImportBoardLayout}
							/>
						</div>

						{item.type === "folder" && (
							<Droppable droppableId={`folder-${item.id}`} type="explorer-item">
								{(droppableProvided, dropSnapshot) => {
									return (
									<div
										ref={droppableProvided.innerRef}
										{...droppableProvided.droppableProps}
										className={`
											  min-h-1
												${item.children?.length ? "pl-4" : ""} 
												${dropSnapshot.isDraggingOver
												? "bg-stone-100 dark:bg-stone-700 border-2 border-dashed border-stone-300 dark:border-stone-600 rounded-md py-2 mx-2"
												: !item.children?.length
													? "py-2 mx-2 border-2 border-dashed border-stone-300 dark:border-stone-600/50 rounded-md"
													: ""
											}
											`}
									>
										{open && item.children?.map((child, childIndex) => (
											<ExplorerItemRow
												key={child.id}
												item={child}
												path={[...path, item.id]}
												index={childIndex}
												onNavigate={onNavigate}
												onAddClick={onAddClick}
												onEditClick={onEditClick}
												onDeleteClick={onDeleteClick}
												exportBoardLayout={exportBoardLayout}
												setMainDashboard={setMainDashboard}
												updatePinnedBoard={updatePinnedBoard}
												importBoardLayout={importBoardLayout}
											/>
										))}
										{droppableProvided.placeholder}
										{dropSnapshot.isDraggingOver && (
											<div className="text-sm text-stone-500 dark:text-stone-400 text-center">
												Drop items here
											</div>
										)}
									</div>
								)}
							}
							</Droppable>
						)}
					</div>
				</div>
			)}
		</Draggable>
	);
}
