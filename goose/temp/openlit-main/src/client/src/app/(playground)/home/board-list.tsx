import { useCallback, useEffect, useState } from "react";
import { Board } from "@/types/manage-dashboard";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { ChevronsUpDown, LayoutDashboard } from "lucide-react";

export default function BoardList({ dashboardId }: { dashboardId: string | null }) {
	const [boards, setBoards] = useState<Board[]>([]);
	const { fireRequest } = useFetchWrapper();

	const fetchBoards = useCallback(() => {
		fireRequest({
			url: "/api/manage-dashboard/board?home=true",
			requestType: "GET",
			successCb: (response) => {
				if (response?.data) {
					setBoards(response.data);
				}
			},
			failureCb: (error) => {
				setBoards([]);
			},
		});
	}, [fireRequest]);

	useEffect(() => {
		fetchBoards();
	}, [fetchBoards]);

	if (boards.length === 0) {
		return null;
	}

	return (
		<div className="flex gap-2">
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button variant="outline" className="flex gap-2 justify-start group-data-[state=close]:justify-center p-[calc(0.625rem-1px)] overflow-hidden text-stone-500 dark:text-stone-300 hover:bg-stone-700 dark:hover:bg-stone-600 hover:text-white font-normal w-full h-auto py-1.5">
						<LayoutDashboard className="size-3" />
						<div className="grid flex-1 text-left text-xs leading-tight group-data-[state=close]:hidden text-ellipsis overflow-hidden whitespace-nowrap">
							<span className="truncate font-medium">Switch Dashboard</span>
						</div>
						<ChevronsUpDown className={`size-4 block group-data-[state=close]:hidden shrink-0 self-center`} />
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent
					className="w-(--radix-dropdown-menu-trigger-width) min-w-56 rounded-lg"
					side="bottom"
					align="end"
					sideOffset={4}
				>
					<DropdownMenuLabel className="font-normal">
						Pinned Dashboard
					</DropdownMenuLabel>
					<DropdownMenuSeparator />
					<DropdownMenuGroup className="gap-1 flex flex-col w-full max-h-40 overflow-y-auto">
						{boards.map((board) => {
							if (dashboardId === board.id || (!dashboardId && board.isMainDashboard)) {
								return null;
							}
							return (
								<DropdownMenuItem className={"text-xs gap-2"} key={board.id}>
									<Link key={board.id} href={`?dashboardId=${board.id}`}>
										{board.title}
									</Link>
								</DropdownMenuItem>
							);
						})}
					</DropdownMenuGroup>
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
};