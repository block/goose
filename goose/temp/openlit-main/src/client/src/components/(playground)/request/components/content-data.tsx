import { isNil } from "lodash";
import JsonViewer from "@/components/common/json-viewer";

export default function ContentDataItem({
	dataKey,
	dataValue,
}: {
	dataKey: string;
	dataValue?: string;
}) {
	return (
		<div
			className={`grid ${
				dataValue ? "grid-cols-2" : ""
			} px-4 py-2 group cursor-pointer dark:bg-stone-700 dark:border-stone-800 dark:last:border-stone-800 odd:bg-stone-200/[0.4] even:bg-stone-200/[0.8] dark:odd:bg-stone-700/[0.4] dark:even:bg-stone-700/[0.8]`}
		>
			<div className="break-all pr-2 text-stone-500 dark:text-stone-300">
				{dataKey}
			</div>
			{!(isNil(dataValue) || dataValue === "") && (
				<div className="break-all pl-2 group-hover:text-stone-950  dark:group-hover:text-stone-100">
					<JsonViewer value={dataValue} />
				</div>
			)}
		</div>
	);
}
