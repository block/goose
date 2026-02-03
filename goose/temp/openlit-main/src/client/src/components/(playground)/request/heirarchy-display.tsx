import { useRequest } from "./request-context";

import React, { useEffect, useState } from "react";
import { FolderTree, Zap, Clock, Activity, Minus, Plus } from "lucide-react";
import {
	findSpanInHierarchyLodash,
	getNormalizedTraceAttribute,
} from "@/helpers/client/trace";
import { TraceHeirarchySpan } from "@/types/trace";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { toast } from "sonner";
import { TraceMapping } from "@/constants/traces";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ResizeablePanel } from "@/components/ui/resizeable-panel";

interface TreeNodeProps {
	span: TraceHeirarchySpan;
	level: number;
	isLast?: boolean;
	parentPath?: boolean[];
}

export function TreeNode({ span, level, isLast = false, parentPath = [] }: TreeNodeProps) {
	const [request, updateRequest] = useRequest();
	const [isExpanded, setIsExpanded] = useState(true);
	const hasChildren = span.children && span.children.length > 0;
	const isSelected = request?.spanId === span.SpanId;

	const onClick = () => {
		if (request?.spanId !== span.SpanId) {
			updateRequest({
				spanId: span.SpanId,
			});
		}
	};

	const toggleExpanded = (e: React.MouseEvent) => {
		e.stopPropagation();
		setIsExpanded(!isExpanded);
	};

	const duration = parseFloat(
		getNormalizedTraceAttribute("requestDuration", span.Duration) as string
	).toFixed(2);

	const getSpanTypeIcon = (spanName: string) => {
		if (spanName.includes('browser_action')) return <Activity className="h-3.5 w-3.5" />;
		if (spanName.includes('model_request')) return <Zap className="h-3.5 w-3.5" />;
		if (spanName.includes('execute_task')) return <Clock className="h-3.5 w-3.5" />;
		return <Zap className="h-3.5 w-3.5" />;
	};

	const getTimingColor = () => {
		const time = parseFloat(duration);
		if (time > 10) return "text-red-600 dark:text-red-400";
		if (time > 5) return "text-yellow-600 dark:text-yellow-400";
		if (time > 1) return "text-blue-600 dark:text-blue-400";
		return "text-green-600 dark:text-green-400";
	};

	const CONNECTION_LINES_CLASSES = "absolute left-3 bg-stone-400 dark:bg-stone-700"

	return (
		<div className="relative">
			{/* Connection lines */}
			{/* {level > 0 && renderConnectionLines()} */}
			<div className="absolute left-0 top-0 bottom-0 flex">
				{parentPath.map((showLine, index) => (
					<div key={index} className="w-6 relative">
						{showLine && (
							<div className={`${CONNECTION_LINES_CLASSES} top-0 bottom-0 w-px`} />
						)}
					</div>
				))}

				{level > 0 && (
					<div className="w-6 relative">
						{/* Vertical line */}
						<div className={`${CONNECTION_LINES_CLASSES} top-0 h-6 w-px`} />
						{/* Horizontal line */}
						<div className={`${CONNECTION_LINES_CLASSES} top-6 w-3 h-px`} />
						{/* Corner connector for non-last items */}
						{!isLast && (
							<div className={`${CONNECTION_LINES_CLASSES} top-6 bottom-0 w-px`} />
						)}
					</div>
				)}
			</div>

			{/* Node content */}
			<div
				className={`flex items-center gap-3 py-2 pr-4 rounded-md transition-colors cursor-pointer group ${level === 0 ? 'ml-0' : `ml-${(level + 1) * 6}`
					}
				 ${isSelected
						? "bg-primary/[0.05] dark:bg-primary/[0.05] text-primary dark:text-primary"
						: "hover:bg-stone-200/[0.5] dark:hover:bg-stone-800 text-stone-700 dark:text-stone-300"
					}
				`}
				style={{ marginLeft: level > 0 ? `${(level + 1) * 24}px` : '0' }}
				onClick={onClick}
				title={span.SpanName}
			>
				{/* Expand/collapse button for parents */}
				{hasChildren && (
					<button
						onClick={toggleExpanded}
						className={`flex items-center justify-center w-5 h-5 rounded border
							 ${isSelected
								? "text-primary/[0.5] dark:text-primary/[0.5] border-primary/[0.5] dark:border-primary/[0.5]"
								: "text-stone-400 dark:text-stone-700 border-stone-400 dark:border-stone-700 "
							}
							`}
						aria-label={isExpanded ? 'Collapse' : 'Expand'}
					>
						{isExpanded ? (
							<Minus className="w-3 h-3" />
						) : (
							<Plus className="w-3 h-3" />
						)}
					</button>
				)}

				{/* Icon */}
				<div className={`flex items-center justify-center w-6 h-6 rounded
					 ${isSelected
						? "bg-primary/[0.2] dark:bg-primary/[0.2] text-primary dark:text-primary"
						: "bg-stone-100 dark:bg-stone-800 text-stone-600 dark:text-stone-400"
					}`}>
					{getSpanTypeIcon(span.SpanName)}
				</div>

				{/* Title */}
				<span className={`text-sm flex-1 overflow-hidden text-ellipsis whitespace-nowrap`}>
					{span.SpanName}
				</span>

				{/* Timing */}
				<span className={`text-sm ${getTimingColor()}`}>
					{duration}{TraceMapping.requestDuration.valueSuffix}
				</span>
			</div>

			{/* Children */}
			{hasChildren && isExpanded && (
				<div className="relative">
					{span?.children!.map((child, index) => {
						const isLastChild = index === span.children!.length - 1;

						return (
							<TreeNode
								key={child.SpanId}
								span={child}
								level={level + 1}
								isLast={isLastChild}
								parentPath={[...parentPath, !isLast]}
							/>
						);
					})}
				</div>
			)}
		</div>
	);
}

const DEFAULT_WIDTH = 46;

export default function HeirarchyDisplay() {
	const [request] = useRequest();
	const { data, fireRequest, isLoading, error } = useFetchWrapper();
	const [accordionValue, setAccordionValue] = useState("");

	useEffect(() => {
		if (!isLoading) {
			setAccordionValue("debug");
		}
	}, [isLoading]);

	const typedData =
		(data as { record: TraceHeirarchySpan; err?: string }) || {};

	useEffect(() => {
		if (
			!error &&
			!findSpanInHierarchyLodash(typedData.record || {}, request?.spanId) &&
			request?.spanId &&
			!isLoading
		) {
			fireRequest({
				requestType: "GET",
				url: `/api/metrics/request/span/${request?.spanId}/heirarchy`,
				failureCb: (err?: string) => {
					toast.error(err || `Cannot connect to server!`, {
						id: "heirarchy-fetch",
					});
				},
			});
		}
	}, [request, typedData, isLoading, error]);

	const { record } = typedData;
	if (isLoading || typedData.err || !record || !record.SpanId) {
		return null;
	}

	return (
		<ResizeablePanel
			defaultWidth={accordionValue ? 450 : DEFAULT_WIDTH}
			minWidth={DEFAULT_WIDTH}
			maxWidth={700}
			handlePosition="left"
			className="absolute left-0 top-0 -translate-x-full h-full bg-stone-100 dark:bg-stone-900 border border-stone-200 border-t-0 dark:border-stone-800 border-r-0 shadow-lg"
		>
			<Accordion type="single" collapsible className="flex flex-1 h-full" value={accordionValue}>
				<AccordionItem value="debug" className="border-0 flex flex-1 w-full">
					<AccordionTrigger
						className="flex flex-col items-center gap-3 px-3 py-6 hover:no-underline hover:bg-stone-100 dark:hover:bg-stone-900 [&[data-state=open]]:bg-stone-100 dark:[&[data-state=open]]:bg-stone-900/50 [&[data-state=open]>svg]:rotate-90 [&[data-state=closed]>svg]:rotate-[-90deg] border-r border-stone-200 dark:border-stone-700 transition-colors"
						onClick={() => setAccordionValue(accordionValue === "debug" ? "" : "debug")}
					>
						<div className="flex flex-col items-center gap-3">
							<FolderTree className="h-5 w-5 text-stone-600 dark:text-stone-400" />
							<span className="text-sm font-semibold [writing-mode:vertical-lr] rotate-180 transform text-stone-700 dark:text-stone-300">
								Span Hierarchy
							</span>
							{isLoading && (
								<div className="flex flex-col items-center gap-1">
									<div className="w-6 h-6 border-2 border-stone-300 border-t-stone-600 rounded-full animate-spin" />
									<span className="text-xs text-stone-500 [writing-mode:vertical-lr] rotate-180">
										Loading...
									</span>
								</div>
							)}
						</div>
					</AccordionTrigger>
					<AccordionContent className="data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down transition-all h-full pb-0" parentClassName="h-full w-full">
						<ScrollArea className="h-full w-full">
							<div className="p-3">
								<div className="mb-3 pb-2 border-b border-stone-200 dark:border-stone-700">
									<h3 className="text-sm font-semibold text-stone-900 dark:text-stone-100">
										Trace Execution Flow
									</h3>
									<p className="text-xs text-stone-500 dark:text-stone-400 mt-1">
										Click spans to view details
									</p>
								</div>
								<TreeNode span={record} level={0} />
							</div>
						</ScrollArea>
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</ResizeablePanel>
	);
}