import { getFilterDetails, getUpdateFilter } from "@/selectors/filter";
import { useRootStore } from "@/store";
import { useCallback, useEffect, useRef } from "react";
import { REFRESH_RATE_TYPE, getTimeLimitObject } from "@/store/filter";
import { usePathname } from "next/navigation";
import { ChevronsUpDown, TimerReset } from "lucide-react";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import { DropdownMenu, DropdownMenuCheckboxItem, DropdownMenuContent, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";

const REFRESH_RATE_EVENT = "refresh-rate";

const PAGES_ENABLED_FOR_REFRESH_RATE =
	/^\/home$|^\/dashboard$|^\/requests$|^\/exceptions$|^\/d\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

const refreshTimes = {
	[REFRESH_RATE_TYPE["30s"]]: 30 * 1000,
	[REFRESH_RATE_TYPE["1m"]]: 60 * 1000,
	[REFRESH_RATE_TYPE["5m"]]: 5 * 60 * 1000,
	[REFRESH_RATE_TYPE["15m"]]: 15 * 60 * 1000,
};
const getRefreshTime = (key: keyof typeof REFRESH_RATE_TYPE) =>
	refreshTimes[key] || 0;

const REFRESH_RATE_TABS: { key: string; label: string }[] = Object.keys(
	REFRESH_RATE_TYPE
).map((k: string) => ({
	key: k,
	label: REFRESH_RATE_TYPE[k as keyof typeof REFRESH_RATE_TYPE],
}));

const RefreshRate = () => {
	const posthog = usePostHog();
	const filter = useRootStore(getFilterDetails);
	const updateFilter = useRootStore(getUpdateFilter);
	const refreshRateTimer = useRef<NodeJS.Timeout>();
	const pathname = usePathname();

	const handleChange = (key: string) => {
		updateFilter("refreshRate", key);
		posthog?.capture(CLIENT_EVENTS.REFRESH_RATE_CHANGE, {
			rate: key,
		});
	};

	const intervalCallback = useCallback(() => {
		const refreshCustomEvent = new CustomEvent(REFRESH_RATE_EVENT);
		document.dispatchEvent(refreshCustomEvent);
	}, []);

	useEffect(() => {
		const refreshTime = getRefreshTime(filter.refreshRate);
		if (refreshTime > 0) {
			refreshRateTimer.current = setInterval(intervalCallback, refreshTime);
		} else {
			clearInterval(refreshRateTimer.current);
			refreshRateTimer.current = undefined;
		}

		return () => {
			clearInterval(refreshRateTimer.current);
			refreshRateTimer.current = undefined;
		};
	}, [filter.refreshRate]);

	const updateEndTime = () => {
		const timeLimit = getTimeLimitObject(filter.timeLimit.type, "") as {
			end: Date;
		};
		updateFilter("timeLimit.end", timeLimit.end);
	};

	const isRefreshRateEnabled =
		PAGES_ENABLED_FOR_REFRESH_RATE.test(pathname);

	useEffect(() => {
		if (isRefreshRateEnabled) {
			document.addEventListener(REFRESH_RATE_EVENT, updateEndTime);
		}
		return () => {
			document.removeEventListener(REFRESH_RATE_EVENT, updateEndTime);
		};
	}, [pathname]);

	if (!isRefreshRateEnabled) return null;

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button variant="outline" className="flex gap-4 shrink-0 justify-start group-data-[state=close]:justify-center p-1 overflow-hidden text-stone-500 dark:text-stone-100 hover:bg-stone-600 dark:hover:bg-stone-600 hover:text-stone-100 font-normal h-auto w-auto ml-auto">
					<TimerReset className={`size-3 shrink-0`} />
					<span className="block text-ellipsis overflow-hidden whitespace-nowrap grow text-xs">{filter.refreshRate}</span>
					<ChevronsUpDown className={`size-3 block shrink-0`} />
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent className="w-56" side="right" align="start">
				<DropdownMenuLabel>Refresh Rate</DropdownMenuLabel>
				<DropdownMenuSeparator />
				{REFRESH_RATE_TABS.map(({ label, key }) => (
					<DropdownMenuCheckboxItem
						key={key}
						checked={key === filter.refreshRate}
						onCheckedChange={() => handleChange(key)}
					>
						<div className="flex items-start text-muted-foreground ">
							<span className="font-medium text-foreground">
								{label}
							</span>
						</div>
					</DropdownMenuCheckboxItem>
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	);
};

export default RefreshRate;
