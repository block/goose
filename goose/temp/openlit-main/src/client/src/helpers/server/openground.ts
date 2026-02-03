export type STATS_ERROR = "none" | "partial" | "all";

export function generateOpengroundStats(requestMeta: any, responseMeta: any) {
	const stats: Record<any, any> = {};
	try {
		stats.prompt = requestMeta.prompt;
		stats.errors = responseMeta
			.filter(([err]: [any, any]) => !!err)
			.map(([err]: [any, any]) => err);
		stats.totalProviders = responseMeta.selectedProviders.length;
		stats.minCostProvider = Infinity;
		stats.minResponseTime = Infinity;
		stats.minCompletionTokens = Infinity;
		(requestMeta.selectedProviders as any[]).forEach(({ provider }, index) => {
			const [, response]: [any, any] = responseMeta?.[index] || [];
			if (response?.evaluationData) {
				if (stats.minCostProvider > response.evaluationData.cost) {
					stats.minCostProvider = provider;
					stats.minCost = response.evaluationData.cost;
				}

				if (stats.minResponseTime > response.evaluationData.responseTime) {
					stats.minResponseTimeProvider = provider;
					stats.minResponseTime = response.evaluationData.responseTime;
				}

				if (
					stats.minCompletionTokens > response.evaluationData.completionTokens
				) {
					stats.minCompletionTokensProvider = provider;
					stats.minCompletionTokens = response.evaluationData.completionTokens;
				}
			}
		});
	} catch {}

	return stats;
}

export function parseOpengroundData(data: any) {
	try {
		const updatedData = data;
		updatedData.responseMeta = JSON.parse(data.responseMeta);
		updatedData.requestMeta = JSON.parse(data.requestMeta);
		updatedData.stats = JSON.parse(data.stats);
		return updatedData;
	} catch (e) {
		return data;
	}
}
