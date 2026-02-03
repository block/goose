import IntermediateState from "@/components/(playground)/intermediate-state";
import { CSSProperties, ReactNode } from "react";
import { fill } from "lodash";
import { objectEntries } from "@/utils/object";
import { Columns } from "./columns";
import { noop } from "@/utils/noop";
import { PRIMARY_BACKGROUND } from "@/constants/common-classes";

const RowWrapper = ({
	children,
	className = "",
	onClick,
	style,
}: {
	children: ReactNode;
	className?: string;
	onClick?: (item: any) => void;
	style?: CSSProperties;
}) => (
	<div className={`grid w-full ${className}`} onClick={onClick} style={style}>
		{children}
	</div>
);

const ColumnRowItem = ({
	children,
	className = "",
	style,
}: {
	children: ReactNode;
	className?: string;
	style?: CSSProperties;
}) => {
	return (
		<div
			className={`flex-shrink-0 border-b dark:border-stone-800 py-3 px-3 overflow-hidden ${className}`}
			style={style}
		>
			{children}
		</div>
	);
};

const RenderLoader = ({
	columns,
	style,
}: {
	columns: string[];
	style?: CSSProperties;
}) =>
	fill(new Array(5), 1).map((_, index) => (
		<RowWrapper
			key={`loader-row-${index}`}
			className={`animate-pulse`}
			style={style}
		>
			{columns.map((_, index) => (
				<ColumnRowItem
					key={`loader-column-${index}`}
					className="group-last-of-type:border-b-0 cursor-pointer py-4"
				>
					<div className="h-2 w-2/3 bg-stone-800/20 dark:bg-stone-100/20 rounded" />
				</ColumnRowItem>
			))}
		</RowWrapper>
	));

export default function DataTable({
	columns,
	data,
	isFetched,
	isLoading,
	visibilityColumns,
	onClick,
	extraFunctions = {},
}: {
	columns: Columns<any, any>;
	data: any[];
	isFetched: boolean;
	isLoading: boolean;
	visibilityColumns: Record<string, boolean>;
	onClick?: (item: any) => void;
	extraFunctions?: Record<string, any>;
}) {
	const visibleColumns = objectEntries(visibilityColumns)
		.filter(([, value]) => value)
		.map(([keys]) => keys);
	const noData = !data?.length && !isLoading;

	const style: CSSProperties = {
		gridTemplateColumns: `repeat(${visibleColumns.length}, minmax(0, 1fr))`,
	};

	const onClickHandler = (rowItem: any) =>
		typeof onClick === "function" ? onClick(rowItem) : noop();

	return (
		<div className={`flex flex-col w-full overflow-auto scrollbar-hidden border dark:border-stone-800 rounded-md grow ${PRIMARY_BACKGROUND}`}>
			<RowWrapper className={`sticky top-0 z-10`} style={style}>
				{visibleColumns.map((column) => (
					<ColumnRowItem
						key={column}
						className={`group-last-of-type:border-b-0 bg-stone-100 text-stone-500 dark:bg-stone-900 dark:text-stone-400 text-sm overflow-hidden text-ellipsis whitespace-nowrap`}
					>
						{columns[column]?.header()}
					</ColumnRowItem>
				))}
			</RowWrapper>
			<div
				className={`flex flex-col w-full ${
					isFetched && isLoading ? "animate-pulse" : ""
				}`}
			>
				{(!isFetched || (isLoading && !data?.length)) && (
					<RenderLoader columns={visibleColumns} style={style} />
				)}
				{data?.map((rowItem, index) => {
					return (
						<RowWrapper
							key={`row-${index}`}
							className={`group text-sm text-stone-700 dark:text-stone-300`}
							onClick={() => onClickHandler(rowItem)}
							style={style}
						>
							{visibleColumns.map((column, cIdx) => (
								<ColumnRowItem
									key={`row-${index}-column-${cIdx}`}
									className={`group-hover:text-stone-700 group-hover:bg-stone-100 dark:group-hover:text-stone-300  dark:group-hover:bg-stone-800 cursor-pointer hover:[&_svg]:text-stone-950 dark:hover:[&_svg]:text-stone-100`}
								>
									{columns[column]?.cell({
										row: rowItem,
										extraFunctions,
									})}
								</ColumnRowItem>
							))}
						</RowWrapper>
					);
				})}
				{noData && <IntermediateState type="nodata" />}
			</div>
		</div>
	);
}
