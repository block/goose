"use client";

import { useEffect } from "react";
import { usePathname, useSearchParams } from "next/navigation";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";

export default function NavigationEvents() {
	const posthog = usePostHog();
	const pathname = usePathname();
	const searchParams = useSearchParams();

	useEffect(() => {
		const url = `${pathname}?${searchParams}`;
		posthog?.capture(CLIENT_EVENTS.PAGE_VISITED, {
			url,
		});
	}, [pathname, searchParams]);

	return null;
}
