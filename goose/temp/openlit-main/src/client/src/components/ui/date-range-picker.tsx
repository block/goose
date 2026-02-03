"use client";

import * as React from "react";
import { addDays, format } from "date-fns";
import { Calendar as CalendarIcon } from "lucide-react";
import { DateRange } from "react-day-picker";

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover";

export function DatePickerWithRange({
	className,
	onCustomDateChange,
	selectedDate,
}: React.HTMLAttributes<HTMLDivElement> & {
	onCustomDateChange: (start: Date, end: Date) => void;
	selectedDate?: {
		start?: Date;
		end?: Date;
	};
}) {
	const [date, setDate] = React.useState<DateRange | undefined>({
		from: selectedDate?.start || addDays(new Date(), -15),
		to: selectedDate?.end || new Date(),
	});

	const onCustomChange = React.useCallback(() => {
		if (date?.from && date?.to) onCustomDateChange(date?.from, date?.to);
	}, [date]);

	return (
		<Popover>
			<PopoverTrigger asChild>
				<Button
					id="date"
					variant={"outline"}
					className={cn(
						"justify-start text-left font-normal dark:bg-stone-900 dark:text-stone-100 text-xs px-2 py-1 self-start h-full",
						!date && "text-muted-foreground",
						className
					)}
				>
					<CalendarIcon className="mr-2 h-3 w-3" />
					{date?.from ? (
						date.to ? (
							<>
								{format(date.from, "LLL dd, y")} -{" "}
								{format(date.to, "LLL dd, y")}
							</>
						) : (
							format(date.from, "LLL dd, y")
						)
					) : (
						<span>Pick a date</span>
					)}
				</Button>
			</PopoverTrigger>
			<PopoverContent className="w-auto p-0" align="start">
				<Calendar
					initialFocus
					mode="range"
					defaultMonth={date?.from}
					selected={date}
					onSelect={setDate}
					numberOfMonths={2}
				/>
				<div className="flex justify-end px-3 pb-3">
					<Button
						className="w-20"
						variant="default"
						size="xs"
						disabled={!date?.from || !date?.to}
						onClick={onCustomChange}
					>
						Done
					</Button>
				</div>
			</PopoverContent>
		</Popover>
	);
}
