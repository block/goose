export type GridPosition = {
	x: number;
	y: number;
	w: number;
	h: number;
};

export type Board = {
	id: string;
	title: string;
	description: string;
	parentId: string | null;
	isMainDashboard: boolean;
	isPinned: boolean;
	createdAt: string;
	updatedAt: string;
	widgets: BoardWidget[];
	totalWidgets?: number;
	tags: string;
};

export type Folder = {
	id: string;
	title: string;
	description: string;
	createdAt: string;
	updatedAt: string;
	parentId: string | null;
	tags: string;
};

export type BoardWidget = {
	id: string;
	boardId: string;
	widgetId: string;
	createdAt: string;
	updatedAt: string;
	position: GridPosition;
	widget: Widget;
};

export interface Widget {
	id: string;
	title: string;
	description: string;
	type: string;
	properties: Record<string, any>;
	config: Record<string, any>;
	createdAt: string;
	updatedAt: string;
	totalBoards?: number;
}

export interface DatabaseWidget {
	properties: string;
	config: string;
}

export type FolderHeirarchy = Folder & {
	boards: Board[];
	children: DashboardHeirarchy[];
	type: "folder";
};

export type BoardHeirarchy = Board & {
	type: "board";
};

export type DashboardItemType = "folder" | "board";

export interface DashboardHeirarchy {
	id: string;
	title: string;
	description: string;
	isMainDashboard?: boolean;
	isPinned?: boolean;
	tags: string;
	type: DashboardItemType;
	children?: DashboardHeirarchy[];
	parentId?: string | null;
}
