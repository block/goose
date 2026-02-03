"use client";
import Filter from "@/components/(playground)/filter";
import {
	DashboardTypeFilter,
	DashboardTypeGraphContainer,
} from "./dashboard-type";

export default function DashboardPage() {
	return (
		<>
			<div className="flex items-center w-full justify-between mb-4">
				<Filter />
				<DashboardTypeFilter />
			</div>
			<DashboardTypeGraphContainer />
		</>
	);
}
