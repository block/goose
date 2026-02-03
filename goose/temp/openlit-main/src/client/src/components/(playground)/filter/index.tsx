import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { getFilterDetails, getUpdateFilter } from "@/selectors/filter";
import { useRootStore } from "@/store";
import { DatePickerWithRange } from "@/components/ui/date-range-picker";
import { TIME_RANGE_TYPE } from "@/store/filter";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import RefreshRate from "./refresh-rate";

const TIME_RANGE_TABS: { key: string; label: string }[] = Object.keys(
	TIME_RANGE_TYPE
).map((k: string) => ({
	key: k,
	label: TIME_RANGE_TYPE[k as keyof typeof TIME_RANGE_TYPE],
}));

const Filter = ({ className = "" }: { className?: string }) => {
	const posthog = usePostHog();
	const filter = useRootStore(getFilterDetails);
	const updateFilter = useRootStore(getUpdateFilter);

	const handleChange = (key: string) => {
		updateFilter("timeLimit.type", key);
		posthog?.capture(CLIENT_EVENTS.TIME_FILTER_CHANGE, {
			range: key,
		});
	};

	const onCustomDateChange = (start: Date, end: Date) => {
		updateFilter("timeLimit.type", TIME_RANGE_TYPE.CUSTOM, { start, end });
		posthog?.capture(CLIENT_EVENTS.TIME_FILTER_CHANGE, {
			range: TIME_RANGE_TYPE.CUSTOM,
		});
	};

	return (
		<div className={`flex grow gap-4 ${className}`}>
			<Tabs defaultValue={filter.timeLimit.type} onValueChange={handleChange}>
				<TabsList className="p-0 h-[30px] border border-stone-200 dark:border-stone-800">
					{TIME_RANGE_TABS.map(({ label, key }) => (
						<TabsTrigger key={key} value={key} className="py-1.5 text-xs">
							{label}
						</TabsTrigger>
					))}
				</TabsList>
			</Tabs>
			{filter.timeLimit.type === "CUSTOM" && (
				<DatePickerWithRange
					selectedDate={filter.timeLimit}
					onCustomDateChange={onCustomDateChange}
				/>
			)}
			<RefreshRate />
		</div>
	);
};

export default Filter;
