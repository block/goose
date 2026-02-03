import React, { useCallback, useEffect, useState } from "react";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogFooter,
	DialogTitle,
	DialogDescription,
} from "@/components/ui/dialog";

// Icon mapping for widget types
import {
	BarChart,
	LineChart,
	PieChart,
	Table,
	AreaChart,
	Gauge,
	Clock,
	LayoutDashboardIcon,
} from "lucide-react";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { toast } from "sonner";
import { Widget } from "@/types/manage-dashboard";
import Header from "../common/header";
import { useDashboardPageSearch } from "@/selectors/dashboards";
import EmptyState from "../common/empty-state";
import getMessage from "@/constants/messages";

const widgetTypeToIcon = {
	STAT_CARD: Gauge,
	BAR_CHART: BarChart,
	LINE_CHART: LineChart,
	PIE_CHART: PieChart,
	TABLE: Table,
	AREA_CHART: AreaChart,
};

function formatDate(dateString: string) {
	const date = new Date(dateString);
	return date.toLocaleDateString(undefined, {
		year: "numeric",
		month: "short",
		day: "numeric",
	});
}

const WidgetListPage = () => {
	const [widgets, setWidgets] = useState<Widget[]>([]);
	const [deleteDialog, setDeleteDialog] = useState<null | {
		id: string;
		title: string;
	}>(null);
	const [deleting, setDeleting] = useState(false);
	const { fireRequest, isLoading } = useFetchWrapper();
	const pageSearch = useDashboardPageSearch();
	const fetchWidgets = useCallback(() => {
		fireRequest({
			url: "/api/manage-dashboard/widget",
			requestType: "GET",
			successCb: (response) => {
				if (response?.data) {
					setWidgets(response.data);
				}
			},
			failureCb: (error) => {
				toast.error("Failed to load widgets");
				console.error("Error loading widgets:", error);
			},
		});
	}, [fireRequest]);

	useEffect(() => {
		fetchWidgets();
	}, [fetchWidgets]);

	const handleDelete = async (id: string) => {
		setDeleting(true);
		// await deleteWidget(id);
		setWidgets((prev) => prev.filter((w) => w.id !== id));
		setDeleting(false);
		setDeleteDialog(null);
	};

	const filteredWidgets = widgets.filter((widget) => {
		return widget.title.toLowerCase().includes(pageSearch.toLowerCase());
	});

	return (
		<div className="flex flex-col gap-3 grow overflow-y-hidden">
			<Header title="Widgets" />

			{isLoading ? (
				<div className="flex justify-center items-center grow">
					<div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
				</div>
			) : filteredWidgets.length === 0 ? (
				<EmptyState
					title={pageSearch ? getMessage().NO_WIDGETS_YET_SEARCH_TITLE : getMessage().NO_WIDGETS_YET}
					description={pageSearch ? getMessage().NO_WIDGETS_YET_SEARCH_DESCRIPTION : getMessage().NO_WIDGETS_YET_DESCRIPTION}
				/>
			) : (
				<div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4 h-full overflow-y-auto">
					{filteredWidgets.map((widget) => {
						const IconComponent =
							widgetTypeToIcon[
							String(widget.type) as keyof typeof widgetTypeToIcon
							] || Gauge;
						return (
							<Card
								key={widget.id}
								className="group hover:shadow-lg transition-all duration-200 cursor-pointer border border-stone-200 dark:border-stone-800 hover:border-stone-300 dark:hover:border-stone-700"
							>
								<CardHeader className="pb-3">
									<CardTitle className="flex gap-2 items-center text-lg font-semibold text-stone-900 dark:text-stone-300 group-hover:text-stone-600 dark:group-hover:text-stone-200 transition-colors">
										<div className="flex items-start justify-between">
											<IconComponent className="h-5 w-5 text-stone-500 dark:text-stone-400 group-hover:text-stone-600 dark:group-hover:text-stone-300 transition-colors" />
											<Badge className="text-xs ml-2" variant="secondary">
												{widget.type}
											</Badge>
										</div>
										{widget.title}
									</CardTitle>
								</CardHeader>
								<CardContent className="space-y-4">
									<p className="text-sm text-stone-700 dark:text-stone-400 line-clamp-2 shrink-0 h-[40px]">
										{widget.description}
									</p>
									<div className="flex items-center justify-between text-sm text-stone-500 dark:text-stone-400">
										<div className="flex items-center gap-1 text-xs">
											<Clock className="h-3 w-3" />
											<span>Updated {formatDate(widget.updatedAt)}</span>
										</div>
										<div className="flex items-center gap-1">
											<LayoutDashboardIcon className="h-3 w-3" />
											<span>{widget.totalBoards ?? 0} Dashboard(s)</span>
										</div>
									</div>
								</CardContent>
							</Card>
						);
					})}
				</div>
			)}
			<Dialog open={!!deleteDialog} onOpenChange={(open) => !open && setDeleteDialog(null)}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>Delete Widget</DialogTitle>
						<DialogDescription>
							Are you sure you want to delete widget &quot;{deleteDialog?.title}&quot;? This
							action cannot be undone.
						</DialogDescription>
					</DialogHeader>
					<DialogFooter>
						<Button variant="secondary" onClick={() => setDeleteDialog(null)} disabled={deleting}>
							Cancel
						</Button>
						<Button
							variant="destructive"
							onClick={() => handleDelete(deleteDialog!.id)}
							disabled={deleting}
						>
							{deleting ? (
								<div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
							) : (
								"Delete"
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
};

export default WidgetListPage;
