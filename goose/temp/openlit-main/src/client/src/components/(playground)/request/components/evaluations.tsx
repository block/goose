import { Button } from "@/components/ui/button";
import getMessage from "@/constants/messages";
import { Evaluation, EvaluationConfigResponse } from "@/types/evaluation";
import { TransformedTraceRow } from "@/types/trace";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import Link from "next/link";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ChevronDown, ChevronUp } from "lucide-react";
import CodeItem from "./code-item";

function getScoreColor(score: number): string {
	if (score < 0.3)
		return "bg-green-100 dark:bg-green-900 text-green-900 dark:text-green-50";
	if (score < 0.7)
		return "bg-yellow-100 dark:bg-yellow-900 text-yellow-900 dark:text-yellow-50";
	return "bg-red-100 dark:bg-red-900 text-red-900 dark:text-red-50";
}

function EvaluationCard({ evaluation }: { evaluation: Evaluation }) {
	const [isExpanded, setIsExpanded] = useState(false);

	return (
		<Card className={`shadow-none border-none rounded-xs`}>
			<CardHeader
				className="cursor-pointer p-3 dark:text-stone-50 bg-stone-200 dark:bg-stone-800"
				onClick={() => setIsExpanded(!isExpanded)}
			>
				<CardTitle className="flex items-center justify-between text-sm text-stone-700 dark:text-stone-300">
					<span>{evaluation.evaluation}</span>
					<div className="flex items-center space-x-2">
						<Badge
							variant="outline"
							className={`${getScoreColor(evaluation.score)} border-none`}
						>
							Score: {evaluation.score}
						</Badge>
						{isExpanded ? <ChevronUp size={20} /> : <ChevronDown size={20} />}
					</div>
				</CardTitle>
			</CardHeader>
			{isExpanded && (
				<CardContent className="space-y-2 p-3 text-sm bg-stone-200/[0.5] dark:bg-stone-900/[0.5]">
					<p>
						<i>Classification:</i> {evaluation.classification}
					</p>
					<p>
						<i>Explanation:</i> {evaluation.explanation}
					</p>
					<p>
						<i>Verdict:</i> {evaluation.verdict}
					</p>
				</CardContent>
			)}
		</Card>
	);
}

export default function Evaluations({ trace }: { trace: TransformedTraceRow }) {
	const [error, setError] = useState<string | null>(null);
	const {
		data: responseData,
		error: responseErr,
		isLoading,
		fireRequest,
		isFetched,
	} = useFetchWrapper<EvaluationConfigResponse>();

	const evaluationData = responseData?.data;

	const {
		fireRequest: runEvaluationRequest,
		isLoading: isRunEvaluationLoading,
	} = useFetchWrapper();

	const runEvaluation = () => {
		runEvaluationRequest({
			url: `/api/evaluation/${trace.spanId}`,
			requestType: "POST",
			responseDataKey: "data",
			successCb: (data: { success: boolean; error?: string }) => {
				if (data?.success) {
					getEvaluations();
				} else {
					setError(data?.error || getMessage().EVALUATION_RUN_FAILURE);
				}
			},
			failureCb: () => {
				toast.error(getMessage().EVALUATION_RUN_FAILURE);
			},
		});
	};

	const getEvaluations = () => {
		fireRequest({
			url: `/api/evaluation/${trace.spanId}`,
			requestType: "GET",
		});
	};

	useEffect(() => {
		if (!evaluationData?.id) {
			getEvaluations();
		}
	}, [trace.spanId]);

	return (
		<div className="flex flex-col gap-2 px-4">
			{isLoading || !isFetched || isRunEvaluationLoading ? (
				<div className="text-sm text-stone-500 dark:text-stone-300">
					{getMessage().EVALUATION_DATA_LOADING}
				</div>
			) : responseData?.configErr ? (
				<>
					<div className="text-sm text-stone-500 dark:text-stone-300">
						{getMessage().EVALUATION_CONFIG_NOT_SET}
					</div>
					<Button variant="destructive" className="w-fit">
						<Link href="/settings/evaluation">
							{getMessage().EVALUATION_CONFIG_SET}
						</Link>
					</Button>
				</>
			) : responseData?.err || error || responseErr ? (
				<div className="text-sm text-stone-500 dark:text-stone-300">
					<CodeItem
						text={{
							error:
								responseData?.err ||
								error ||
								responseErr ||
								getMessage().EVALUATION_RUN_FAILURE,
						}}
					/>
					<Button
						variant="default"
						className="w-fit bg-primary"
						onClick={runEvaluation}
					>
						{getMessage().EVALUATION_RUN_AGAIN}
					</Button>
				</div>
			) : responseData?.config ? (
				<>
					<div className="text-sm text-stone-500 dark:text-stone-300">
						{getMessage().EVALUATION_NOT_RUN_YET}
					</div>
					<Button
						variant="default"
						className="w-fit bg-primary"
						onClick={runEvaluation}
					>
						{getMessage().EVALUATION_RUN}
					</Button>
				</>
			) : (
				evaluationData?.evaluations?.map((evaluation, index) => (
					<EvaluationCard key={index} evaluation={evaluation} />
				))
			)}
		</div>
	);
}
