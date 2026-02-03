"use client";

import { useEffect } from "react";
import { fetchAndPopulateCurrentUserStore } from "@/helpers/client/user";
import { getIsUserFetched } from "@/selectors/user";
import { useRootStore } from "@/store";

export default function AppInit() {
	const isFetched = useRootStore(getIsUserFetched);
	useEffect(() => {
		if (!isFetched) fetchAndPopulateCurrentUserStore();
	}, [isFetched]);

	return null;
}
