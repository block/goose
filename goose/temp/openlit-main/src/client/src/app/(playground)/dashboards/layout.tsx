"use client";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { FolderIcon, LayoutDashboardIcon, ComponentIcon } from "lucide-react";
import { useSetDashboardPageSearch } from "@/selectors/dashboards";

export default function LayoutManageDashboard({
	children,
}: {
	children: React.ReactNode;
}) {
	const pathname = usePathname();
	const setPageSearch = useSetDashboardPageSearch();
	const resetSearch = () => {
		setPageSearch("");
	}

	return (
		<div className="flex flex-col w-full h-full gap-4">
			{!pathname.includes("/dashboards/board/") && (
				<div className="flex">
					<Tabs defaultValue={pathname.split("/")[2] || "explorer"} className="">
						<TabsList className="p-0 h-[30px] px-[1px]">
							<TabsTrigger value="explorer" className="p-0 w-[100px] text-xs rounded-md">
								<Link
									className="py-1.5 w-full flex items-center justify-center gap-2 px-2"
									href="/dashboards/explorer"
									onClick={resetSearch}
								>
									<FolderIcon className="h-3 w-3" />
									Explorer
								</Link>
							</TabsTrigger>
							<TabsTrigger value="board" className="p-0 w-[100px] text-xs rounded-md">
								<Link className="py-1.5 w-full flex items-center justify-center gap-2 px-2" href="/dashboards/dashboard" onClick={resetSearch}>
									<LayoutDashboardIcon className="h-3 w-3" />
									Dashboard
								</Link>
							</TabsTrigger>
							<TabsTrigger value="widget" className="p-0 w-[100px] text-xs rounded-md">
								<Link className="py-1.5 w-full flex items-center justify-center gap-2 px-2" href="/dashboards/widget" onClick={resetSearch}>
									<ComponentIcon className="h-3 w-3" />
									Widget
								</Link>
							</TabsTrigger>
						</TabsList>
					</Tabs>
				</div>
			)}
			{children}
		</div>
	);
}
