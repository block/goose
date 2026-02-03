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
				className="rounded-md group-hover:text-stone-100 text-stone-700 dark:text-stone-300 block overflow-hidden text-ellipsis"
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
				<div className="truncate" title={row.spanName}>
					{row.spanName}
				</div>
			);
		},
	},
	requestDuration: {
		header: () => TraceMapping.requestDuration.label,
		cell: ({ row }) => {
			const value = `${parseFloat(row.requestDuration).toFixed(3)}${
				TraceMapping.requestDuration.valueSuffix
			}`;
			return (
				<div className="truncate" title={value}>
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
				<div className="truncate" title={row.serviceName}>
					{row.serviceName}
				</div>
			);
		},
		enableHiding: true,
	},
	applicationName: {
		header: () => TraceMapping.applicationName.label,
		cell: ({ row }) => {
			return (
				<div className="truncate" title={row.applicationName}>
					{row.applicationName}
				</div>
			);
		},
		enableHiding: true,
	},
	cost: {
		header: () => TraceMapping.cost.label,
		cell: ({ row }) => {
			const value = `${TraceMapping.cost.valuePrefix}${row.cost}`;
			return (
				<div className="truncate" title={value}>
					{value}
				</div>
			);
		},
		enableHiding: true,
	},
	totalTokens: {
		header: () => TraceMapping.totalTokens.label,
		cell: ({ row }) => {
			return (
				<div className="truncate" title={row.totalTokens}>
					{row.totalTokens}
				</div>
			);
		},
		enableHiding: true,
	},
	model: {
		header: () => TraceMapping.model.label,
		cell: ({ row }) => {
			return (
				<div className="truncate" title={row.model}>
					{row.model}
				</div>
			);
		},
		enableHiding: true,
	},
	system: {
		header: () => TraceMapping.system.label,
		cell: ({ row }) => {
			return (
				<div className="truncate" title={row.system}>
					{row.system}
				</div>
			);
		},
		enableHiding: true,
	},
	vectorCount: {
		header: () => TraceMapping.vectorCount.label,
		cell: ({ row }) => {
			return (
				<div className="truncate" title={row.vectorCount}>
					{row.vectorCount}
				</div>
			);
		},
		enableHiding: true,
	},
};
