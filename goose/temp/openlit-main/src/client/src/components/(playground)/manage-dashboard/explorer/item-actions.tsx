import { DashboardHeirarchy } from "@/types/manage-dashboard";
import { useCallback, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Plus, MoreHorizontal, Edit, Trash2, Download, Upload, PinIcon } from "lucide-react";
import ImportLayoutModal from "./import-layout-modal";

export default function ItemActions({
	item,
	path,
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
	onAddClick: (path: string[]) => void;
	onEditClick: (item: DashboardHeirarchy, path: string[]) => void;
	onDeleteClick: (id: string, path: string[]) => void;
	exportBoardLayout: (id: string) => void;
	setMainDashboard: (id: string) => void;
	updatePinnedBoard: (id: string) => void;
	importBoardLayout: (data: any) => Promise<unknown>;
}) {
	const [isImportModalOpen, setIsImportModalOpen] = useState(false);

	const handleAddClick = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation(); // Prevent event from bubbling up
			onAddClick([...path, item.id]);
		},
		[item.id, path, onAddClick]
	);

	const handleEditClick = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation(); // Prevent event from bubbling up
			onEditClick(item, path);
		},
		[item, path, onEditClick]
	);

	const handleDeleteClick = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation(); // Prevent event from bubbling up
			onDeleteClick(item.id, path);
		},
		[item.id, path, onDeleteClick]
	);

	const handleDownloadClick = useCallback(
		(e: React.MouseEvent) => {
			e.stopPropagation();
			exportBoardLayout(item.id);
		},
		[item.id, exportBoardLayout]
	);

	return (
		<div
			className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity text-stone-500 dark:text-stone-400"
			onClick={(e) => e.stopPropagation()}
		>
			{item.type === "folder" && (
				<Button
					variant="ghost"
					size="icon"
					className="h-8 w-8"
					onClick={handleAddClick}
				>
					<Plus className="h-4 w-4" />
				</Button>
			)}

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button variant="ghost" size="icon" className="h-8 w-8 self-center">
						<MoreHorizontal className="h-4 w-4" />
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="end">
					<DropdownMenuItem onClick={handleEditClick}>
						<Edit className="h-4 w-4 mr-2" />
						Edit details
					</DropdownMenuItem>
					{item.type === "board" ? (
						<>
							<DropdownMenuItem onClick={handleDownloadClick}>
								<Download className="h-4 w-4 mr-2" />
								Export Layout
							</DropdownMenuItem>
							{
								!item.isMainDashboard && (
									<DropdownMenuItem onClick={() => setMainDashboard(item.id)}>
										<Download className="h-4 w-4 mr-2" />
										Set as Main Dashboard
									</DropdownMenuItem>
								)
							}
							<DropdownMenuItem onClick={() => updatePinnedBoard(item.id)}>
								<PinIcon className="h-4 w-4 mr-2" />
								{item.isPinned ? "Unpin" : "Pin"}
							</DropdownMenuItem>
						</>
					) : (
						<DropdownMenuItem onClick={() => setIsImportModalOpen(true)}>
							<Upload className="h-4 w-4 mr-2" />
							Import Layout
						</DropdownMenuItem>
					)}
					<DropdownMenuItem
						className="text-destructive focus:text-destructive"
						onClick={handleDeleteClick}
					>
						<Trash2 className="h-4 w-4 mr-2" />
						Delete
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>

			<ImportLayoutModal
				open={isImportModalOpen}
				onClose={() => setIsImportModalOpen(false)}
				onImport={importBoardLayout as any}
			/>
		</div>
	);
}
