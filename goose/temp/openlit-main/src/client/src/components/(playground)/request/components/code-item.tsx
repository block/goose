import JSONViewer from "@/components/common/json-viewer";

export default function CodeItem({
	label,
	text,
}: {
	label?: string;
	text: unknown;
}) {
	return (
		<div className="flex flex-col space-y-3 group py-4 w-full">
			{label && (
				<span className="text-sm text-stone-500 font-medium dark:text-stone-300">
					{label} :
				</span>
			)}
			<code className="text-sm inline-flex text-left items-center bg-stone-300 text-stone-700 rounded-md p-4 group-hover:text-stone-900 cursor-pointer dark:bg-stone-800 dark:text-stone-200 dark:group-hover:text-stone-100">
				<JSONViewer value={text} />
			</code>
		</div>
	);
}
