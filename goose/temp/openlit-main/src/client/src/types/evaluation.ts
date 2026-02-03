import { EvaluationConfigs } from "@prisma/client";
import { Secret } from "./vault";

export interface EvaluationConfig extends EvaluationConfigs {}

export interface EvaluationConfigInput
	extends Omit<EvaluationConfig, "id" | "databaseConfigId"> {
	id?: string;
	databaseConfigId?: string;
}

export interface EvaluationConfigResponse {
	config: string;
	configErr: string;
	data: EvaluationResponse;
	err: string;
	success?: boolean;
}
export interface Evaluation {
	score: number;
	evaluation: string;
	classification: string;
	explanation: string;
	verdict: string;
}

export interface EvaluationResponse {
	id: string;
	spanId: string;
	createdAt: Date;
	meta: Record<string, any>;
	evaluations: Evaluation[];
}

export interface AutoEvaluationConfig {
	evaluationConfigId: string;
	cronId: string;
}

export type EvaluationConfigWithSecret = EvaluationConfig & { secret: Secret };
