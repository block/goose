import type React from "react";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import type { ColorTheme, TableWidget } from "../types";

interface TableWidgetProps {
	widget: TableWidget;
	data?: any[];
}

const TableWidgetComponent: React.FC<TableWidgetProps> = ({ widget, data }) => {
	if (!data || data.length === 0) {
		return (
			<div className="flex justify-center items-center h-full text-muted-foreground">
				No data available
			</div>
		);
	}

	const columns = Object.keys(data[0]).map((key) => ({
		header: key.charAt(0).toUpperCase() + key.slice(1),
		cell: (row: any) => typeof row[key] === "object" ? JSON.stringify(row[key]) : row[key],
		className: "border-r last:border-r-0",
	}));

	const backgroundColor = `${widget.properties.color}20`;
	const borderColor = `${widget.properties.color}50`;

	return (
		<div className="flex flex-col h-full overflow-auto border rounded-lg"
			style={{
				borderColor,
			}}>
			<Table>
				<TableHeader className="sticky top-0 z-10"
					style={{
						backgroundColor,
					}}>
					<TableRow
						className="border-b"
						style={{
							borderColor,
						}}
					>
						{columns.map((column, index) => (
							<TableHead
								key={index}
								className="px-4 py-2 font-semibold border-r last:border-r-0 h-auto"
								style={{
									color: widget.properties.color as ColorTheme,
									borderColor,
								}}
							>
								{column.header}
							</TableHead>
						))}
					</TableRow>
				</TableHeader>
				<TableBody>
					{data.length === 0 ? (
						<TableRow>
							<TableCell colSpan={columns.length} className="text-center text-stone-500 dark:text-stone-400">
								No results.
							</TableCell>
						</TableRow>
					) : (
						data.map((row, rowIndex) => (
							<TableRow
								key={rowIndex}
								className="border-b last:border-b-0 hover:bg-stone-50/50 dark:hover:bg-stone-800/50"
							>
								{columns.map((column, colIndex) => {
									return (
										<TableCell
											key={`${rowIndex}-${colIndex}`}
											className="border-r last:border-r-0 px-4 py-2 text-stone-600 dark:text-stone-300"
											style={{
												borderColor,
											}}
										>
											{column.cell(row)}
										</TableCell>
									);
								})}
							</TableRow>
						))
					)}
				</TableBody>
			</Table>
		</div>
	);
};

export default TableWidgetComponent;
