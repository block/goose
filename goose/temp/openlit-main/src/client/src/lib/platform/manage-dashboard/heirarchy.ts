import {
	Board,
	DashboardHeirarchy,
	Folder,
	FolderHeirarchy,
} from "@/types/manage-dashboard";
import { getBoards } from "./board";
import { getFolders } from "./folder";

export async function getHeirarchy() {
	const { data: boards = [], err: errBoards } = await getBoards();
	const { data: folders = [], err: errFolders } = await getFolders();

	if (errBoards || errFolders) return { err: errBoards || errFolders };

	return { data: buildHierarchy(boards as Board[], folders as Folder[]) };
}

function buildHierarchy(
	boards: Board[],
	folders: Folder[]
): DashboardHeirarchy[] {
	const folderMap = new Map<string, FolderHeirarchy>();
	const rootNodes: DashboardHeirarchy[] = [];
	const processedFolders = new Set<string>();

	// Initialize folders in map with empty children and boards
	folders.forEach((folder) => {
		folderMap.set(folder.id, { 
			...folder, 
			type: "folder",
			children: [], 
			boards: [],
		});
	});

	// Build folder hierarchy first
	folders.forEach((folder) => {
		if (folder.parentId && folderMap.has(folder.parentId)) {
			// Add this folder as a child to its parent
			folderMap.get(folder.parentId)!.children.push(folderMap.get(folder.id)!);
			processedFolders.add(folder.id);
		}
	});

	// Add only root-level folders to rootNodes
	folders.forEach((folder) => {
		// Only add folders that don't have a parent or whose parent doesn't exist
		// AND haven't been processed as a child of another folder
		if ((!folder.parentId || !folderMap.has(folder.parentId)) && 
			!processedFolders.has(folder.id)) {
			if (folderMap.has(folder.id)) {
				rootNodes.push(folderMap.get(folder.id)!);
			}
		}
	});

	// Assign boards to their respective folders (or root level)
	boards.forEach((board) => {
		const boardNode: DashboardHeirarchy = { 
			id: board.id, 
			title: board.title, 
			description: board.description,
			isMainDashboard: board.isMainDashboard,
			isPinned: board.isPinned,
			type: "board",
			children: [],
			parentId: board.parentId,
			tags: board.tags,
		};
		
		if (board.parentId && folderMap.has(board.parentId)) {
			// Add board to its parent folder
			folderMap.get(board.parentId)!.boards.push(board);
			folderMap.get(board.parentId)!.children.push(boardNode);
		} else {
			// Root-level boards (not inside any folder)
			rootNodes.push(boardNode);
		}
	});

	return rootNodes;
}
