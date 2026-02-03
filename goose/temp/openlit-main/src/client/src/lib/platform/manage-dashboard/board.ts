import { Board, Widget } from "@/types/manage-dashboard";
import { dataCollector } from "../common";
import {
	OPENLIT_BOARD_TABLE_NAME,
	OPENLIT_BOARD_WIDGET_TABLE_NAME,
} from "./table-details";
import getMessage from "@/constants/messages";
import Sanitizer from "@/utils/sanitizer";
import { createWidget, getWidgets } from "./widget";
import { pluck } from "lodash/fp";
import { jsonParse, jsonStringify } from "@/utils/json";

export function getBoardById(id: string) {
	const query = `
		SELECT *
		FROM ${OPENLIT_BOARD_TABLE_NAME} 
		WHERE id = '${Sanitizer.sanitizeValue(id)}'
	`;

	return dataCollector({ query });
}

export function getBoards(isHome?: boolean) {
	let query: string = '';
	if (isHome) {
		query = `
			SELECT b.id, b.title, b.description, b.parent_id AS parentId, 
						 b.is_main_dashboard AS isMainDashboard, 
						 b.is_pinned AS isPinned,
						 b.created_at AS createdAt, 
						 b.updated_at AS updatedAt, b.tags
			FROM ${OPENLIT_BOARD_TABLE_NAME} b
			WHERE b.is_main_dashboard = true OR b.is_pinned = true
			ORDER BY b.updated_at DESC;
		`;
	} else {
		query = `
			SELECT b.id, b.title, b.description, b.parent_id AS parentId, 
						 b.is_main_dashboard AS isMainDashboard, 
						 b.is_pinned AS isPinned,
						 b.created_at AS createdAt, 
						 b.updated_at AS updatedAt, COALESCE(bwc.total_widgets, 0) AS totalWidgets, b.tags
			FROM ${OPENLIT_BOARD_TABLE_NAME} b
			LEFT JOIN (
				SELECT board_id, COUNT(*) AS total_widgets
				FROM ${OPENLIT_BOARD_WIDGET_TABLE_NAME}
				WHERE board_id IS NOT NULL
				GROUP BY board_id
			) AS bwc ON b.id = bwc.board_id
			ORDER BY b.updated_at DESC;
		`;
	}

	return dataCollector({ query });
}

export async function createBoard(board: Board, databaseConfigId?: string) {
	const sanitizedBoard = Sanitizer.sanitizeObject(board);

	const { err, data } = await dataCollector(
		{
			table: OPENLIT_BOARD_TABLE_NAME,
			values: [
				{
					title: sanitizedBoard.title,
					description: sanitizedBoard.description,
					parent_id: sanitizedBoard.parentId,
					is_main_dashboard: sanitizedBoard.isMainDashboard,
					is_pinned: sanitizedBoard.isPinned,
					tags: jsonStringify(sanitizedBoard.tags),
				},
			],
		},
		"insert",
		databaseConfigId
	);

	if (err || !(data as { query_id: string }).query_id)
		return { err: getMessage().BOARD_CREATE_FAILED };

	const { data: data_board, err: err_board } = await dataCollector({
		query: `Select * from ${OPENLIT_BOARD_TABLE_NAME} order by created_at desc limit 1`,
	});


	if (err_board) {
		return { err: getMessage().BOARD_CREATE_FAILED };
	}

	return { data: (data_board as any[])[0] };
}

export async function updateBoard(board: Board & { updateParent?: boolean }) {
	const sanitizedBoard = Sanitizer.sanitizeObject(board);

	const updateValues = [
		sanitizedBoard.title && `title = '${sanitizedBoard.title}'`,
		sanitizedBoard.description &&
		`description = '${sanitizedBoard.description}'`,
		board.updateParent && `parent_id = ${sanitizedBoard.parentId ? `'${sanitizedBoard.parentId}'` : 'NULL'}`,
		sanitizedBoard.tags && `tags = '${jsonStringify(sanitizedBoard.tags)}'`,
	];

	const query = `
		ALTER TABLE ${OPENLIT_BOARD_TABLE_NAME}
		UPDATE 
			${updateValues.filter((e) => e).join(" , ")}
		WHERE id = '${sanitizedBoard.id}'
	`;

	const { err, data } = await dataCollector({ query }, "exec");

	if (err || !(data as { query_id: string }).query_id)
		return { err: getMessage().BOARD_UPDATE_FAILED };

	const { data: boardData } = await getBoardById(sanitizedBoard.id);

	return { data: boardData };
}

export async function deleteBoard(id: string) {
	const query_board_widgets = `
		DELETE FROM ${OPENLIT_BOARD_WIDGET_TABLE_NAME} 
		WHERE board_id = '${Sanitizer.sanitizeValue(id)}'
	`;

	const query = `
		DELETE FROM ${OPENLIT_BOARD_TABLE_NAME} 
		WHERE id = '${Sanitizer.sanitizeValue(id)}'
	`;

	const { err: err_board_widgets } = await dataCollector(
		{ query: query_board_widgets },
		"exec"
	);

	if (err_board_widgets) return { err: getMessage().BOARD_DELETE_FAILED };

	const { err: err_board } = await dataCollector({ query }, "exec");

	if (err_board) return { err: getMessage().BOARD_DELETE_FAILED };

	return { data: getMessage().BOARD_DELETED_SUCCESSFULLY };
}

export async function getBoardLayout(id: string, databaseConfigId?: string) {
	// First get the board details
	const boardQuery = `
		SELECT 
			id AS boardId,
			title AS boardTitle,
			description AS boardDescription,
			is_main_dashboard AS isMainDashboard,
			is_pinned AS isPinned,
			created_at AS boardCreatedAt,
			updated_at AS boardUpdatedAt,
			tags AS tags
		FROM ${OPENLIT_BOARD_TABLE_NAME}
		WHERE id = '${Sanitizer.sanitizeValue(id)}'
	`;

	const { data: boardData, err: boardErr } = await dataCollector({
		query: boardQuery,
	}, "query", databaseConfigId);
	const boardResult = (boardData as any[])?.[0] as {
		boardId: string;
		boardTitle: string;
		boardDescription: string;
		isMainDashboard: boolean;
		isPinned: boolean;
		boardCreatedAt: string;
		boardUpdatedAt: string;
		tags: string;
	};

	if (!boardResult || boardErr)
		return { err: getMessage().BOARD_DATA_NOT_FOUND };

	// Then get the widget mappings with positions
	const widgetMappingsQuery = `
		SELECT 
			bw.id AS boardWidgetId,
			bw.widget_id AS widgetId,
			bw.position,
			bw.created_at AS boardWidgetCreatedAt,
			bw.updated_at AS boardWidgetUpdatedAt
		FROM ${OPENLIT_BOARD_WIDGET_TABLE_NAME} bw
		WHERE bw.board_id = '${Sanitizer.sanitizeValue(id)}'
	`;

	const { data: mappingsData, err: mappingsErr } = await dataCollector({
		query: widgetMappingsQuery,
	}, "query", databaseConfigId);
	const mappingsResult = (mappingsData || []) as Array<{
		boardWidgetId: string;
		widgetId: string;
		position: string;
		boardWidgetCreatedAt: string;
		boardWidgetUpdatedAt: string;
		tags: string;
	}>;

	// Finally get the widget details
	const widgetIds = pluck("widgetId", mappingsResult);

	let widgetsResult: Array<Widget> = [];

	if (widgetIds) {
		const { data: widgetsData } = await getWidgets(widgetIds);
		widgetsResult = (widgetsData || []) as typeof widgetsResult;
	}

	// Create a map of widget details for easy lookup
	const widgetDetailsMap = new Map(widgetsResult.map((w) => [w.id, w]));

	// Construct the board object
	const board: Board = {
		id: boardResult.boardId,
		title: boardResult.boardTitle,
		description: boardResult.boardDescription,
		parentId: null,
		isMainDashboard: boardResult.isMainDashboard,
		isPinned: boardResult.isPinned,
		createdAt: boardResult.boardCreatedAt,
		updatedAt: boardResult.boardUpdatedAt,
		widgets: [],
		tags: boardResult.tags,
	};

	// Construct the layout config format
	type LayoutItem = {
		i: string;
		x: number;
		y: number;
		w: number;
		h: number;
	};

	type LayoutConfig = {
		layouts: {
			lg: LayoutItem[];
		};
		widgets: {
			[key: string]: Widget;
		};
	};

	const layoutConfig: LayoutConfig = {
		layouts: { lg: [] },
		widgets: {},
	};

	// Map widgets and create layout config
	board.widgets = mappingsResult.map((mapping) => {
		const widgetDetails = widgetDetailsMap.get(mapping.widgetId);
		const position = JSON.parse(mapping.position || "{}");

		// Add to layouts
		layoutConfig.layouts.lg.push({
			i: mapping.widgetId,
			x: position.x,
			y: position.y,
			w: position.w,
			h: position.h,
		});

		// Add to widgets map
		const widgetData = {
			id: mapping.widgetId,
			title: widgetDetails?.title || "",
			description: widgetDetails?.description || "",
			type: widgetDetails?.type || "",
			properties: widgetDetails?.properties || {},
			config: widgetDetails?.config || {},
			createdAt: widgetDetails?.createdAt || "",
			updatedAt: widgetDetails?.updatedAt || "",
		} as const;

		layoutConfig.widgets[mapping.widgetId] = widgetData;

		// Return the board widget mapping
		return {
			id: mapping.boardWidgetId,
			boardId: board.id,
			widgetId: mapping.widgetId,
			position: position || {},
			createdAt: mapping.boardWidgetCreatedAt,
			updatedAt: mapping.boardWidgetUpdatedAt,
			widget: widgetData,
		};
	});

	return {
		data: {
			...board,
			...layoutConfig,
		},
	};
}

export async function updateBoardLayout(boardId: string, layoutConfig: any, databaseConfigId?: string) {
	// First, get all existing widget mappings for this board
	const getExistingWidgetsQuery = `
			SELECT widget_id, id
			FROM ${OPENLIT_BOARD_WIDGET_TABLE_NAME}
			WHERE board_id = '${Sanitizer.sanitizeValue(boardId)}'
		`;

	const { data: existingWidgetsData } = await dataCollector({
		query: getExistingWidgetsQuery,
	}, "query", databaseConfigId);
	const existingWidgets =
		(existingWidgetsData as { widget_id: string; id: string }[]) || [];

	// Create a map of existing widget IDs for quick lookup
	const existingWidgetMap = new Map();
	existingWidgets.forEach((widget) => {
		existingWidgetMap.set(widget.widget_id, widget.id);
	});

	// Create a set of widget IDs from the new layout config
	const newWidgetIds = new Set();
	layoutConfig.layouts.lg.forEach((layout: { i: string }) => {
		const widget = layoutConfig.widgets[layout.i];
		if (widget && widget.id) {
			newWidgetIds.add(widget.id);
		}
	});

	// Delete widgets that are no longer in the layout
	for (const existingWidget of existingWidgets) {
		if (!newWidgetIds.has(existingWidget.widget_id)) {
			const deleteQuery = `
					DELETE FROM ${OPENLIT_BOARD_WIDGET_TABLE_NAME}
					WHERE id = '${Sanitizer.sanitizeValue(existingWidget.id)}'
					AND board_id = '${Sanitizer.sanitizeValue(boardId)}'
				`;
			await dataCollector({ query: deleteQuery }, "exec", databaseConfigId);
		}
	}

	// Update or insert widget mappings
	for (const layout of layoutConfig.layouts.lg) {
		const widget = layoutConfig.widgets[layout.i];
		if (widget && widget.id) {
			const position = {
				x: layout.x,
				y: layout.y,
				w: layout.w,
				h: layout.h,
			};

			if (existingWidgetMap.has(widget.id)) {
				// Update existing widget mapping
				const updateQuery = `
						ALTER TABLE ${OPENLIT_BOARD_WIDGET_TABLE_NAME}
						UPDATE 
							position = '${JSON.stringify(position)}',
							updated_at = NOW()
						WHERE id = '${Sanitizer.sanitizeValue(existingWidgetMap.get(widget.id))}'
					`;

				await dataCollector({ query: updateQuery }, "exec", databaseConfigId);
			} else {
				// Insert new widget mapping
				await dataCollector(
					{
						table: OPENLIT_BOARD_WIDGET_TABLE_NAME,
						values: [
							{
								board_id: Sanitizer.sanitizeValue(boardId),
								widget_id: Sanitizer.sanitizeValue(widget.id),
								position: JSON.stringify(position),
							},
						],
					},
					"insert",
					databaseConfigId
				);
			}
		}
	}

	return { data: getMessage().BOARD_LAYOUT_UPDATED_SUCCESSFULLY, err: null };
}

export async function setMainDashboard(boardId: string) {
	// Set all boards to is_main_dashboard = false
	const unsetQuery = `
		ALTER TABLE ${OPENLIT_BOARD_TABLE_NAME}
		UPDATE is_main_dashboard = false
		WHERE is_main_dashboard = true
	`;
	await dataCollector({ query: unsetQuery }, "exec");

	// Set the selected board to is_main_dashboard = true
	const setQuery = `
		ALTER TABLE ${OPENLIT_BOARD_TABLE_NAME}
		UPDATE is_main_dashboard = true
		WHERE id = '${Sanitizer.sanitizeValue(boardId)}'
	`;
	const { err, data } = await dataCollector({ query: setQuery }, "exec");

	if (err || !(data as { query_id: string }).query_id)
		return { err: err?.toString() || getMessage().BOARD_UPDATE_FAILED };

	return { data: getMessage().BOARD_UPDATED_SUCCESSFULLY };
}

export async function getMainDashboard(layout?: boolean, databaseConfigId?: string) {
	const query = `
		SELECT id, is_main_dashboard AS isMainDashboard FROM ${OPENLIT_BOARD_TABLE_NAME}
		WHERE is_main_dashboard = true
	`;

	const { data: mainDashboardData, err: mainDashboardErr } =
		await dataCollector({ query }, "query", databaseConfigId);

	if (mainDashboardErr || !(mainDashboardData as any[])[0])
		return { err: mainDashboardErr?.toString() || getMessage().MAIN_DASHBOARD_NOT_FOUND };

	if (layout) {
		return getBoardLayout((mainDashboardData as any[])[0].id, databaseConfigId);
	}

	return { data: (mainDashboardData as any[])[0], err: null };
}

// TODO: fix the type of data
export async function importBoardLayout(data: any, databaseConfigId?: string) {
	const mainDashboard = await getMainDashboard(false, databaseConfigId);

	const boardData: Partial<Board> = {
		title: data.title,
		description: data.description,
		isPinned: data.isPinned,
		isMainDashboard: mainDashboard.data?.isMainDashboard ? false : data.isMainDashboard,
		tags: data.tags ? jsonParse(data.tags) : [],
	};

	// Create the board first
	const boardResult = await createBoard(boardData as Board, databaseConfigId);

	if ('err' in boardResult) {
		return { err: boardResult.err };
	}

	const newBoardId = boardResult.data.id;

	// Update the board layout with widgets and their positions
	const layoutConfig = {
		layouts: data.layouts,
		widgets: data.widgets
	};

	const widgetIdMap = new Map();

	const updatedWidgets = Object.values(layoutConfig.widgets).map((widget: any) => {
		const newWidgetId = crypto.randomUUID();
		widgetIdMap.set(widget.id, newWidgetId);

		return {
			...widget,
			id: newWidgetId,
		};
	});

	const updatedLayouts = layoutConfig.layouts.lg.map((layout: any) => {
		return {
			...layout,
			i: widgetIdMap.get(layout.i)
		}
	});

	await Promise.all(updatedWidgets.map(async (widget: any) => {
		return await createWidget(widget, databaseConfigId);
	}));

	const layoutConfigData = {
		layouts: {
			lg: updatedLayouts
		},
		widgets: updatedWidgets.reduce((acc: any, widget: any) => {
			acc[widget.id] = widget;
			return acc;
		}, {}),
	}


	const { data: layoutData, err: layoutErr } = await updateBoardLayout(newBoardId, layoutConfigData, databaseConfigId);


	if (layoutData) {
		return { data: boardResult.data };
	}

	return { err: layoutErr || getMessage().BOARD_IMPORT_FAILED };
}

export async function updatePinnedBoard(boardId: string) {
	// Set the selected board to is_pinned = true
	const setQuery = `
		ALTER TABLE ${OPENLIT_BOARD_TABLE_NAME}
		UPDATE is_pinned = NOT is_pinned
		WHERE id = '${Sanitizer.sanitizeValue(boardId)}'
	`;
	const { err, data } = await dataCollector({ query: setQuery }, "exec");

	if (err || !(data as { query_id: string }).query_id)
		return { err: err?.toString() || getMessage().BOARD_UPDATE_FAILED };

	return { data: getMessage().BOARD_UPDATED_SUCCESSFULLY };
}

export async function isBoardTableEmpty(databaseConfigId?: string) {
	const query = `
		SELECT CAST(COUNT(*) AS INTEGER) as count FROM ${OPENLIT_BOARD_TABLE_NAME}
	`;

	const { data: boardCount, err: boardCountErr } =
		await dataCollector({ query }, "query", databaseConfigId);

	if (boardCountErr) {
		return { err: boardCountErr, data: false };
	}

	return { data: (boardCount as any[])[0].count === 0, err: null };
}