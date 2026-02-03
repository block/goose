"use client";

import React, { useEffect, useState } from "react";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { ScrollArea } from "@/components/ui/scroll-area";
import { AlertCircle, CheckCircle2, Terminal } from "lucide-react";

interface QueryDebuggerProps {
	data?: any;
	error?: string;
	isLoading?: boolean;
}

const QueryDebugger: React.FC<QueryDebuggerProps> = ({
	data,
	error,
	isLoading,
}) => {
	const [accordionValue, setAccordionValue] = useState("");
	
	useEffect(() => {
		if (!isLoading) {
			setAccordionValue("debug");
		}
	}, [isLoading]);

	return (
		<div className="absolute left-0 top-0 -translate-x-full flex h-full bg-stone-100 dark:bg-stone-900 border border-stone-200 border-t-0 dark:border-stone-900 border-r-0 text-stone-800 dark:text-stone-300">
			<Accordion type="single" collapsible className="flex" value={accordionValue}>
				<AccordionItem value="debug" className="border-0 flex">
					<AccordionTrigger className="flex flex-col items-center gap-2 px-2 py-4 hover:no-underline hover:bg-muted/80 [&[data-state=open]]:bg-muted [&[data-state=open]>svg]:rotate-90 [&[data-state=closed]>svg]:rotate-[-90deg] border-r border-stone-200 dark:border-stone-700" onClick={() => setAccordionValue(accordionValue === "debug" ? "" : "debug")}>
						<div className="flex flex-col items-center gap-2">
							{error ? (
								<AlertCircle className="h-4 w-4 text-error" />
							) : data ? (
								<CheckCircle2 className="h-4 w-4 text-success" />
							) : (
								<Terminal className="h-4 w-4 text-muted-foreground" />
							)}
							<span className="text-sm font-medium [writing-mode:vertical-lr] rotate-180 transform">Query Debug Console</span>
							{isLoading && (
								<span className="text-xs text-muted-foreground [writing-mode:vertical-lr] rotate-180 animate-pulse">
									Running...
								</span>
							)}
						</div>
					</AccordionTrigger>
					<AccordionContent className="data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down transition-all h-full">
						<ScrollArea className="h-full w-[400px] bg-background p-4">
							{isLoading ? (
								<div className="flex items-center justify-center h-full">
									<span className="text-sm text-muted-foreground">
										Running query...
									</span>
								</div>
							) : error ? (
								<Alert variant="destructive">
									<AlertDescription>{error}</AlertDescription>
								</Alert>
							) : data ? (
								<pre className="text-sm whitespace-pre-wrap">
									{JSON.stringify(data, null, 2)}
								</pre>
							) : (
								<div className="flex items-center justify-center h-full">
									<span className="text-sm text-muted-foreground">
										No query results to display
									</span>
								</div>
							)}
						</ScrollArea>
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</div>
	);
};

export default QueryDebugger;
