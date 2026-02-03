import { DatabaseWidget, Widget } from "@/types/manage-dashboard";
import { jsonParse, jsonStringify } from "@/utils/json";
import Sanitizer from "@/utils/sanitizer";

export function normalizeWidgetToClient(widget: DatabaseWidget) {
	const unsanitizedWidget = unsanitizeWidget(widget);
	return {
		...widget,
		properties: unsanitizedWidget.properties ? jsonParse(unsanitizedWidget.properties) : unsanitizedWidget.properties,
		config: unsanitizedWidget.config ? jsonParse(unsanitizedWidget.config) : unsanitizedWidget.config,
	};
}

export function normalizeWidgetToServer(widget: Widget) {
	return {
		...widget,
		properties: widget.properties ? jsonStringify(widget.properties) : widget.properties,
		config: widget.config ? jsonStringify(widget.config) : widget.config,
	};
}

export function unsanitizeWidget(widget: DatabaseWidget) {
	return {
		...widget,
		config: widget.config
			.replace(/\n/g, '\\n')
			.replace(/\t/g, '\\t'),
	};
}

export function sanitizeWidget(widget: Widget) {
	const sanitizedWidget = Sanitizer.sanitizeObject(widget);

	if (sanitizedWidget.config?.query) {
		sanitizedWidget.config.query = sanitizedWidget.config.query.replace(/''/g, "'")
			.replace(/\\'/g, "'").replace(/\\n/g, '\n')
			.replace(/\\t/g, '\t');
	}

	if (sanitizedWidget.config?.content) {
		sanitizedWidget.config.content = sanitizedWidget.config.content.replace(/''/g, "'")
			.replace(/\\'/g, "'").replace(/\\n/g, '\n')
			.replace(/\\t/g, '\t');
	}

	return sanitizedWidget;
}

export function escapeSingleQuotes(input: string) {
	return input.replace(/'([^']*)'/g, "''$1''");
}