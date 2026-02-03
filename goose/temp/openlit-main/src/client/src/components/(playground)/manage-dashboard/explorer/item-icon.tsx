import { DashboardItemType } from "@/types/manage-dashboard";
import { Folder, FolderOpen, LayoutDashboardIcon } from "lucide-react";

export default function ItemIcon({
	type,
	open,
}: {
	type: DashboardItemType;
	open?: boolean;
}) {
	if (type === "folder") {
		return open ? (
			<FolderOpen className="h-4 w-4" />
		) : (
			<Folder className="h-4 w-4" />
		);
	}
	return <LayoutDashboardIcon className="h-4 w-4" />;
}
