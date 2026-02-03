import { importBoardLayout, isBoardTableEmpty } from "@/lib/platform/manage-dashboard/board";
import llmDashboard from "../seed-data/openlit-dashboard-LLM-dashboard-layout.json";
import vectorDbDashboard from "../seed-data/openlit-dashboard-Vector-DB-layout.json";
import gpuDashboard from "../seed-data/openlit-dashboard-GPU-dashboard-layout.json";

export default async function CreateCustomDashboardsSeed(databaseConfigId?: string) {
	console.log(
		`********* Seeding Dashboards *********`
	);

	const { data: isBoardTableEmptyData, err: isBoardTableEmptyErr } = await isBoardTableEmpty(databaseConfigId);

	if (isBoardTableEmptyErr) {
		console.log(
			`********* Error checking if board table is empty *********`
		);
		console.log(isBoardTableEmptyErr);
		return;
	}

	if (!isBoardTableEmptyData) {
		console.log(
			`********* Board Table is not empty hence skipping seeding *********`
		);
		return;
	}

	const { err: llmDashboardErr } = await importBoardLayout(llmDashboard, databaseConfigId);
	const { err: vectorDbDashboardErr } = await importBoardLayout(vectorDbDashboard, databaseConfigId);
	const { err: gpuDashboardErr } = await importBoardLayout(gpuDashboard, databaseConfigId);

	if (llmDashboardErr || vectorDbDashboardErr || gpuDashboardErr) {
		console.log(
			`********* Seeding Dashboards Failed *********`
		);
		console.log(llmDashboardErr, vectorDbDashboardErr, gpuDashboardErr);
		return;
	}

	console.log(
		`********* Seeding Dashboards Completed *********`
	);
}