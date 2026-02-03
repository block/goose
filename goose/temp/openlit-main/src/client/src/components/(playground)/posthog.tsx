"use client";
import {
	POSTHOG_API_HOST,
	POSTHOG_API_KEY,
} from "@/constants/posthog";
import { PostHogConfig } from "posthog-js";
import { PostHogProvider } from "posthog-js/react";
import { ReactNode } from "react";

const POSTHOG_OPTIONS: Partial<PostHogConfig> = {
	api_host: POSTHOG_API_HOST,
	autocapture: false,
};

export default function CustomPostHogProvider({
	children,
	telemetryEnabled,
}: {
	children: ReactNode;
	telemetryEnabled: boolean;
}) {
	if (telemetryEnabled) {
		return (
			<PostHogProvider
				apiKey={POSTHOG_API_KEY}
				options={POSTHOG_OPTIONS}
			>
				{children}
			</PostHogProvider>
		);
	}

	return children as JSX.Element;
}
