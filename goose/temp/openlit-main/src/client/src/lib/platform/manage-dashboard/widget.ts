import { DatabaseWidget, Widget } from "@/types/manage-dashboard";
import { dataCollector, MetricParams } from "../common";
import { OPENLIT_BOARD_WIDGET_TABLE_NAME, OPENLIT_WIDGET_TABLE_NAME } from "./table-details";
import Sanitizer from "@/utils/sanitizer";
import getMessage from "@/constants/messages";
import {
	normalizeWidgetToClient,
	sanitizeWidget,
	escapeSingleQuotes,
} from "@/helpers/server/widget";
import mustache from "mustache";

import { jsonStringify } from "@/utils/json";

export async function getWidgetById(id: string) {
	const query = `
		SELECT id, title, description, widget_type AS type, created_at AS createdAt, updated_at AS updatedAt, properties,
			config
		FROM ${OPENLIT_WIDGET_TABLE_NAME} 
		WHERE id = '${Sanitizer.sanitizeValue(id)}'
	`;

	const { data, err } = await dataCollector({ query });

	if (err) {
		return { err: err.toString() || getMessage().WIDGET_FETCH_FAILED };
	}

	return { data: normalizeWidgetToClient((data as DatabaseWidget[])[0]) };
}

export async function getWidgets(widgetIds?: string[]) {
	let query = "";
	if (!widgetIds || widgetIds.length === 0) {
		query = `
			SELECT w.id, w.title, w.description, w.widget_type AS type, w.properties,
			w.config, w.created_at AS createdAt, w.updated_at AS updatedAt, COUNT(DISTINCT bw.board_id) as totalBoards
		FROM ${OPENLIT_WIDGET_TABLE_NAME} w
		LEFT JOIN ${OPENLIT_BOARD_WIDGET_TABLE_NAME} bw ON w.id = bw.widget_id
		GROUP BY w.id, w.title, w.description, w.widget_type, w.properties, w.config, w.created_at, w.updated_at
		ORDER BY w.updated_at DESC
		`;
	} else {
		query = `
			SELECT w.id, w.title, w.description, w.widget_type AS type, w.properties,
			w.config, w.created_at AS createdAt, w.updated_at AS updatedAt
		FROM ${OPENLIT_WIDGET_TABLE_NAME} w
		${widgetIds
				? `WHERE id IN (${widgetIds
					.map((id) => `'${Sanitizer.sanitizeValue(id)}'`)
					.join(",")})`
				: ""
			}
		`;
	}

	const { data, err } = await dataCollector({ query });

	if (err) {
		return { err: err.toString() || getMessage().WIDGET_FETCH_FAILED };
	}

	return { data: (data as Array<DatabaseWidget>).map(normalizeWidgetToClient) };
}

export async function createWidget(widget: Widget, databaseConfigId?: string) {
	const sanitizedWidget = sanitizeWidget(widget);

	const { err, data } = await dataCollector(
		{
			table: OPENLIT_WIDGET_TABLE_NAME,
			values: [
				{
					id: sanitizedWidget.id,
					title: sanitizedWidget.title,
					description: sanitizedWidget.description,
					widget_type: sanitizedWidget.type,
					properties: JSON.stringify(sanitizedWidget.properties || {}),
					config: JSON.stringify(sanitizedWidget.config || {}),
				},
			],
		},
		"insert",
		databaseConfigId
	);

	// Extract the ID from the response
	const queryId = (data as { query_id: string })?.query_id;

	if (err || !queryId) {
		return { err: err || getMessage().WIDGET_CREATE_FAILED };
	}

	// Get the created widget to return its ID
	if (queryId) {
		const result = await dataCollector({
			query: `SELECT id, title, description, widget_type AS type, created_at AS createdAt, properties,
			config, updated_at AS updatedAt FROM ${OPENLIT_WIDGET_TABLE_NAME} ORDER BY created_at DESC LIMIT 1`,
		}, "query", databaseConfigId);

		if (
			!result.err &&
			result.data &&
			Array.isArray(result.data) &&
			result.data.length > 0
		) {
			return {
				data: {
					...normalizeWidgetToClient(result.data[0] as DatabaseWidget),
				},
			};
		}
	}

	return { err: getMessage().WIDGET_CREATE_FAILED };
}

export async function updateWidget(widget: Widget) {
	const sanitizedWidget = sanitizeWidget(widget);

	const updateValues = [
		sanitizedWidget.title && `title = '${sanitizedWidget.title}'`,
		sanitizedWidget.description &&
		`description = '${sanitizedWidget.description}'`,
		sanitizedWidget.type && `widget_type = '${sanitizedWidget.type}'`,
		sanitizedWidget.properties &&
		`properties = '${jsonStringify(sanitizedWidget.properties)}'`,
		sanitizedWidget.config &&
		`config = '${escapeSingleQuotes(jsonStringify(sanitizedWidget.config))}'`,
		`updated_at = NOW()`,
	];

	const query = `
		ALTER TABLE ${OPENLIT_WIDGET_TABLE_NAME}
		UPDATE 
			${updateValues.filter((e) => e).join(" , ")}
		WHERE id = '${sanitizedWidget.id}'
	`;

	const { err, data } = await dataCollector({ query }, "exec");

	if (err || !(data as { query_id: string }).query_id) {
		return { err: err || getMessage().WIDGET_UPDATE_FAILED };
	}

	return { data: getMessage().WIDGET_UPDATED_SUCCESSFULLY };
}

export function deleteWidget(id: string) {
	const query = `
		DELETE FROM ${OPENLIT_WIDGET_TABLE_NAME} 
		WHERE id = '${Sanitizer.sanitizeValue(id)}'
	`;

	return dataCollector({ query }, "exec");
}

export async function runWidgetQuery(
	widgetId: string,
	{
		userQuery,
		filter,
	}: {
		userQuery?: string;
		filter: MetricParams;
	}
) {
	const { data: widget, err: widgetErr } = await getWidgetById(widgetId);

	if (widgetErr || !widget) {
		return { err: getMessage().WIDGET_FETCH_FAILED };
	}

	const query = userQuery
		? userQuery
		: widget.config?.query || "";

	const exactQuery = mustache.render(query, { filter });

	const { data, err } = await dataCollector({ query: exactQuery, enable_readonly: true });

	if (err) {
		return { err: err || getMessage().WIDGET_RUN_FAILED };
	}

	return { data };
}
