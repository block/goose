import { ChevronsUpDown, Database, Plus } from "lucide-react";
import { getDatabaseConfigList } from "@/selectors/database-config";
import { useRootStore } from "@/store";
import { useEffect } from "react";
import {
	changeActiveDatabaseConfig,
	fetchDatabaseConfigList,
} from "@/helpers/client/database-config";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import { DropdownMenu, DropdownMenuCheckboxItem, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { ICON_CLASSES } from "@/constants/sidebar";
import Link from "next/link";

export default function DatabaseConfigSwitch() {
	const posthog = usePostHog();
	const list = useRootStore(getDatabaseConfigList) || [];
	const activeDatabase = list.find((item) => !!item.isCurrent);
	const onClickItem = (id: string) => {
		changeActiveDatabaseConfig(id, () => {
			posthog?.capture(CLIENT_EVENTS.DB_CONFIG_ACTION_CHANGE);
		});
	};

	useEffect(() => {
		fetchDatabaseConfigList((data: any[]) => {
			posthog?.capture(CLIENT_EVENTS.DB_CONFIG_LIST, {
				count: data.length,
			});
		});
	}, []);

	if (!activeDatabase) return null;

	return (
		<DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" className="flex gap-2 shrink-0 justify-start group-data-[state=close]:justify-center p-[calc(0.625rem-1px)] overflow-hidden text-stone-500 dark:text-stone-300 hover:bg-stone-700 dark:hover:bg-stone-600 hover:text-white font-normal">
					<Database className={`${ICON_CLASSES} shrink-0`} />
					<span className="block group-data-[state=close]:hidden text-ellipsis overflow-hidden whitespace-nowrap grow">{activeDatabase?.name}</span>
					<ChevronsUpDown className={`size-4 block group-data-[state=close]:hidden shrink-0`} />
				</Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-56" side="right" align="start">
        <DropdownMenuLabel>Databases</DropdownMenuLabel>
        <DropdownMenuSeparator />
				{list.map((item) => (
					<DropdownMenuCheckboxItem
					key={item.id}
          checked={item.id === activeDatabase.id}
          onCheckedChange={() => onClickItem(item.id)}
        >
          <div className="flex items-start text-muted-foreground ">
							<div className="grid">
								<p>
									<span className="font-medium text-foreground">
										{item.name}
									</span>
								</p>
								<p className="text-xs" data-description>
									{item.environment}
								</p>
							</div>
						</div>
        </DropdownMenuCheckboxItem>
				))}
        <DropdownMenuSeparator />
				<DropdownMenuItem className="py-1.5 pl-8 pr-2">
					<Link href="/settings/database-config" className=" flex items-center">
						Add New Config
					</Link>
				</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
	);
}