export const exportBoardLayout = (id: string) => {
	window.location.href = `/api/manage-dashboard/board/${id}/layout/export`;
};