import asaw from "@/utils/asaw";
import OpenAIProvider from "./providers/openai";
import AnthropicProvider from "./providers/anthropic";
import CohereProvider from "./providers/cohere";
import MistralProvider from "./providers/mistral";
import { omit } from "lodash";
import prisma from "@/lib/prisma";
import { getCurrentUser } from "@/lib/session";
import { getDBConfigByUser } from "@/lib/db-config";
import {
	generateOpengroundStats,
	parseOpengroundData,
} from "@/helpers/server/openground";

async function createOpenGroundRequest(dataObject: {
	responseMeta: any;
	requestMeta: any;
}) {
	try {
		const user = await getCurrentUser();
		const [, dbConfig] = await asaw(getDBConfigByUser(true));
		if (user && dbConfig) {
			await prisma.openGround.create({
				data: {
					requestMeta: JSON.stringify(dataObject.requestMeta),
					responseMeta: JSON.stringify(dataObject.responseMeta),
					createdByUserId: user.id,
					databaseConfigId: dbConfig.id,
					stats: JSON.stringify(
						generateOpengroundStats(
							dataObject.requestMeta,
							dataObject.responseMeta
						)
					),
				},
			});
		}
	} catch (e) {
		console.error("Error in saving openground request", e);
	}
}

type evaluateParams = {
	prompt: string;
	selectedProviders: any[];
};

export async function evaluate(params: evaluateParams) {
	const requestMeta: {
		prompt: string;
		selectedProviders: { provider: string; config: any }[];
	} = {
		prompt: params.prompt,
		selectedProviders: [],
	};
	const responseMeta = await Promise.all(
		params.selectedProviders.map(({ provider, config }) => {
			const objectParams = { ...config, prompt: params.prompt };
			requestMeta.selectedProviders.push({
				provider,
				config: omit(objectParams, ["api_key", "token"]),
			});
			switch (provider) {
				case "openai":
					return asaw(OpenAIProvider.evaluate(objectParams));
				case "anthropic":
					return asaw(AnthropicProvider.evaluate(objectParams));
				case "cohere":
					return asaw(CohereProvider.evaluate(objectParams));
				case "mistral":
					return asaw(MistralProvider.evaluate(objectParams));
				default:
					return ["Type not supported yet!", null];
			}
		})
	);

	createOpenGroundRequest({
		requestMeta,
		responseMeta,
	});

	return responseMeta;
}

export async function getAllOpengroundRequests(
	limit: number = 100,
	offset: number = 0
) {
	return await prisma.openGround.findMany({
		skip: offset,
		take: limit,
		select: {
			id: true,
			stats: true,
			createdByUser: {
				select: {
					name: true,
					email: true,
				},
			},
			databaseConfig: {
				select: {
					name: true,
				},
			},
		},
	});
}

export async function getOpengroundRequest(id: string) {
	const [err, data] = await asaw(
		prisma.openGround.findFirst({
			where: {
				id,
			},
			include: {
				createdByUser: {
					select: {
						name: true,
						email: true,
					},
				},
				databaseConfig: {
					select: {
						name: true,
					},
				},
			},
		})
	);

	if (err) {
		return [err];
	}

	return [, parseOpengroundData(data)];
}
