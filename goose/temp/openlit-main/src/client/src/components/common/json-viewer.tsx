import { jsonParse } from "@/utils/json";
import {
	JsonViewer,
	JsonViewerProps,
	defineEasyType,
} from "@textea/json-viewer";
import { isInteger } from "lodash";

const integerType = defineEasyType<string>({
	is: (value) => {
		const changedValue = parseInt(value as string, 10);
		return isInteger(changedValue);
	},
	type: "number",
	colorKey: "base0C",
	Renderer: (props) => parseInt(props.value, 10),
});

const objectType = defineEasyType<string>({
	is: (value) => {
		if (!/^\[|\{/.test(value as string)) {
			return false;
		}
		return jsonParse(value as string);
	},
	type: "object",
	colorKey: "base0E",
	Renderer: (props) => props.value,
});

const booleanType = defineEasyType<string>({
	is: (value) => {
		return value === "true" || value === "false";
	},
	type: "bool",
	colorKey: "base0E",
	Renderer: (props) => {
		return `${!!(props.value === "true")}`;
	},
});

export default function JSONViewer(props: JsonViewerProps) {
	return (
		<JsonViewer
			value={props.value}
			className={`bg-transparent dark:bg-transparent text-success dark:text-success ${props.className}`}
			enableClipboard={props.enableClipboard || true}
			displayDataTypes={props.displayDataTypes || false}
			displaySize={props.displaySize || false}
			rootName={props.rootName || false}
			valueTypes={[integerType, objectType, booleanType]}
			maxDisplayLength={20}
		/>
	);
}
