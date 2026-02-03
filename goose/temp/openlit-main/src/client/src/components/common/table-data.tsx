import { get } from "lodash";
import IntermediateState from "@/components/(playground)/intermediate-state";

export type TableDataProps = {
	className?: string;
	columns: {
		key?: string;
		className?: string;
		header: string;
		render?: (data: any, extraFunction?: any) => any;
	}[];
	isFetched?: boolean;
	isLoading?: boolean;
	data: any[];
	idKey?: string;
	extraFunction?: any;
};

export default function TableData({
	className,
	columns,
	isFetched,
	isLoading,
	data,
	idKey = "id",
	extraFunction,
}: TableDataProps) {
	return (
		<div
			className={`flex flex-col w-full relative overflow-hidden rounded-md border dark:border-stone-500 ${
				className || ""
			}`}
		>
			<div className="grid grid-cols-12 border-b text-stone-500 text-sm bg-stone-200 dark:bg-stone-800 dark:text-stone-400 dark:border-stone-500">
				{columns.map((column, index) => {
					return (
						<div
							key={index}
							className={`items-center py-2 px-3 ${column.className}`}
						>
							{column.header}
						</div>
					);
				})}
			</div>
			<div
				className={`flex flex-col w-full text-sm text-left relative overflow-auto ${
					isFetched && isLoading ? "animate-pulse" : ""
				}`}
			>
				{(!isFetched || (isLoading && !data?.length)) && (
					<div className={`flex items-center justify-center py-4 px-3`}>
						<div className="h-2 w-full bg-stone-100 dark:bg-stone-900 rounded col-span-1" />
					</div>
				)}
				{data.map((item: any, index: number) => {
					return (
						<div
							className={`grid grid-cols-12 ${
								index === data.length - 1
									? ""
									: "border-b dark:border-stone-500"
							} items-center text-stone-600 dark:text-stone-300 group`}
							key={item[idKey]}
						>
							{columns.map(({ key, render, className }, index) => (
								<div
									className={`${className} items-center py-2 px-3 text-ellipsis overflow-hidden`}
									key={`${item[idKey]}-column-${index}`}
								>
									{render ? render(item, extraFunction) : get(item, key || "")}
								</div>
							))}
						</div>
					);
				})}
				{!data?.length && !isLoading && isFetched && (
					<IntermediateState type="nodata" classNames="!p-3 text-xs" />
				)}
			</div>
		</div>
	);
}
