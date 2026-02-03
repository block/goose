export type DashboardStore = {
	page: {
		search: string;
	},
	setPageSearch: (search: string) => void;
};
