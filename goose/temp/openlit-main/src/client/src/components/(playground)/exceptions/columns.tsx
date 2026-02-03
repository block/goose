import { Columns } from "@/components/data-table/columns";
import { Badge } from "@/components/ui/badge";
import { TraceMapping } from "@/constants/traces";
import { TraceMappingKeyType, TransformedTraceRow } from "@/types/trace";
import { CalendarDays } from "lucide-react";

export const columns: Columns<TraceMappingKeyType, TransformedTraceRow> = {
	id: {
		header: () => TraceMapping.id.label,
		cell: ({ row }) => (
			<div
				className="rounded-md block overflow-hidden text-ellipsis"
				title={row.id}
			>
				{row.id}
			</div>
		),
		enableHiding: false,
	},
	time: {
		header: () => TraceMapping.time.label,
		cell: ({ row }) => {
			return (
				<div className="flex space-x-2 items-center" title={row.time}>
					<CalendarDays size="16" />
					<span className="truncate font-medium">{row.time}</span>
				</div>
			);
		},
		enableHiding: false,
	},
	spanName: {
		header: () => TraceMapping.spanName.label,
		cell: ({ row }) => {
			return (
				<div
					className="block items-center overflow-hidden text-ellipsis"
					title={row.spanName}
				>
					{row.spanName}
				</div>
			);
		},
		enableHiding: true,
	},
	requestDuration: {
		header: () => TraceMapping.requestDuration.label,
		cell: ({ row }) => {
			const value = `${parseFloat(row.requestDuration).toFixed(3)}${
				TraceMapping.requestDuration.valueSuffix
			}`;
			return (
				<div
					className="block items-center overflow-hidden text-ellipsis"
					title={value}
				>
					{value}
				</div>
			);
		},
		enableHiding: true,
	},
	serviceName: {
		header: () => TraceMapping.serviceName.label,
		cell: ({ row }) => {
			return (
				<div
					className="block items-center overflow-hidden text-ellipsis"
					title={row.serviceName}
				>
					{row.serviceName}
				</div>
			);
		},
		enableHiding: true,
	},
	deploymentType: {
		header: () => TraceMapping.deploymentType.label,
		cell: ({ row }) => {
			return (
				<div
					className="block items-center overflow-hidden text-ellipsis"
					title={row.deploymentType}
				>
					{row.deploymentType}
				</div>
			);
		},
		enableHiding: true,
	},
	exceptionType: {
		header: () => TraceMapping.exceptionType.label,
		cell: ({ row }) => {
			return (
				<div
					className="block items-center overflow-hidden text-ellipsis"
					title={row.exceptionType}
				>
					{row.exceptionType}
				</div>
			);
		},
		enableHiding: true,
	},
};
