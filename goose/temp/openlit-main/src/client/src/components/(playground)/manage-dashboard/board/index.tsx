import { useState, useEffect, useCallback } from "react";
import { LayoutDashboardIcon, Clock, ComponentIcon, PinIcon } from "lucide-react";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { toast } from "sonner";
import { Board } from "@/types/manage-dashboard";
import { jsonParse } from "@/utils/json";
import { useRouter } from "next/navigation";
import Header from "../common/header";
import { useDashboardPageSearch } from "@/selectors/dashboards";
import EmptyState from "../common/empty-state";
import getMessage from "@/constants/messages";

function formatDate(dateString: string) {
	const date = new Date(dateString);
	return date.toLocaleDateString(undefined, {
		year: "numeric",
		month: "short",
		day: "numeric",
	});
}

export default function BoardList() {
	const [boards, setBoards] = useState<Board[]>([]);
	const pageSearch = useDashboardPageSearch();
	const { fireRequest, isLoading } = useFetchWrapper();
	const router = useRouter();
	const fetchBoards = useCallback(() => {
		fireRequest({
			url: "/api/manage-dashboard/board",
			requestType: "GET",
			successCb: (response) => {
				if (response?.data) {
					setBoards(response.data);
				}
			},
			failureCb: (error) => {
				toast.error("Failed to load boards");
				console.error("Error loading boards:", error);
			},
		});
	}, [fireRequest]);

	useEffect(() => {
		fetchBoards();
	}, [fetchBoards]);

	const navigateToBoard = useCallback((boardId: string) => {
		router.push(`/d/${boardId}`);
	}, []);

	const filteredBoards = boards.filter((board) => {
		return board.title.toLowerCase().includes(pageSearch.toLowerCase());
	});

	return (
		<div className="flex flex-col gap-3 grow overflow-y-hidden">
			<Header title="Dashboard" />
			<div className="grow rounded-sm overflow-y-auto">
				{isLoading ? (
					<div className="flex justify-center items-center py-8">
						<div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
					</div>
				) : filteredBoards.length === 0 ? (
					<EmptyState
						title={pageSearch ? getMessage().NO_DASHBOARDS_YET_SEARCH_TITLE : getMessage().NO_DASHBOARDS_YET}
						description={pageSearch ? getMessage().NO_DASHBOARDS_YET_SEARCH_DESCRIPTION : getMessage().NO_DASHBOARDS_YET_DESCRIPTION}
					/>
				) : (
					<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
						{filteredBoards.map((board) => (
							<Card
								key={board.id}
								className="group hover:shadow-lg transition-all duration-200 cursor-pointer border border-stone-200 dark:border-stone-800 hover:border-stone-300 dark:hover:border-stone-700"
								onClick={() => navigateToBoard(board.id)}
							>
								<CardHeader className="pb-3">
									<CardTitle className="flex gap-2 items-center text-lg font-semibold text-stone-900 dark:text-stone-300 group-hover:text-stone-600 dark:group-hover:text-stone-200 transition-colors">
										<LayoutDashboardIcon className="h-5 w-5 text-stone-500 dark:text-stone-400 group-hover:text-stone-600 dark:group-hover:text-stone-300 transition-colors" />
										<span className="grow shrink-0">{board.title}</span>
										<div className="flex items-end justify-between">
											{board.isMainDashboard && (
												<Badge className="text-xs" variant="secondary">
													Main Dashboard
												</Badge>
											)}
											{board.isPinned && (
												<Badge className="text-xs" variant="secondary">
													<PinIcon className="h-3 w-3" />
												</Badge>
											)}
										</div>
									</CardTitle>
								</CardHeader>
								<CardContent className="space-y-4">
									<p className="text-sm text-stone-700 dark:text-stone-400 line-clamp-2 shrink-0 h-[40px]">
										{board.description}
									</p>
									<div className="flex items-center gap-2">
										{jsonParse(board.tags)?.map?.((tag: string) => (
											<Badge key={tag}>{tag}</Badge>
										))}
									</div>
									<div className="flex items-center justify-between text-sm text-stone-500 dark:text-stone-300">
										<div className="flex items-center gap-1 text-xs">
											<Clock className="h-3 w-3" />
											<span>Updated {formatDate(board.updatedAt)}</span>
										</div>
										<div className="flex items-center gap-1">
											<ComponentIcon className="h-3 w-3" />
											<span>{board.totalWidgets ?? 0} widgets</span>
										</div>
									</div>
								</CardContent>
							</Card>
						))}
					</div>
				)}
			</div>
		</div>
	);
}
