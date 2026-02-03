import { FilterType } from "@/types/store/filter";
import { omit } from "lodash";

export const getFilterParamsForDashboard = (filter: FilterType) => {
	return omit(filter, [
		"limit",
		"offset",
		"selectedConfig",
		"sorting",
		"refreshRate",
	]);
};
