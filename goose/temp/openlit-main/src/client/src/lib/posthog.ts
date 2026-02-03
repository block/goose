import {
	POSTHOG_API_HOST,
	POSTHOG_API_KEY,
} from "@/constants/posthog";
import { jsonStringify } from "@/utils/json";
import { consoleLog } from "@/utils/log";
import { randomUUID } from "crypto";
/* This is to force the node to use IPv4 for posthog events. Although this sets this config globally but right now it's only used for posthog events. If any problem occurs in future, we can remove this or use a undici for the fetch wrapper */
import { setDefaultAutoSelectFamily } from 'node:net';
setDefaultAutoSelectFamily(false);

export default class PostHogServer {
	static distinctId: string = randomUUID();

	static async capture(options: {
		event: string,
		timestamp?: Date,
		properties?: Record<string, unknown>,
	}) {
		const telemetryEnabled = process.env.TELEMETRY_ENABLED !== "false";

		try {
			if (telemetryEnabled) {
				await fetch(
					`${POSTHOG_API_HOST}/capture/`,
					{
						method: "POST",
						headers: {
							"Content-Type": "application/json",
						},
						body: jsonStringify({
							api_key: POSTHOG_API_KEY,
							...options,
							distinctId: this.distinctId,
							properties: {
								...(options.properties || {}),
								isServer: true,
							},
						}),
					},
				);
			}
		} catch (error) {
			consoleLog("Error capturing telemetry events:", error);
		}
	}

	static async fireEvent({
		event,
		properties = {},
		startTimestamp,
	}: {
		event: string;
		properties?: Record<string, unknown>;
		startTimestamp: number;
	}) {
		await PostHogServer.capture({
			event,
			timestamp: new Date(startTimestamp),
			properties: {
				...properties,
				responseTime: Date.now() - startTimestamp,
			},
		});
	}
}
