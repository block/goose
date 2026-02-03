"use client";

import SecretForm from "@/components/(playground)/vault/form";
import FormBuilder from "@/components/common/form-builder";
import { FormBuilderEvent } from "@/types/form";
import { Button } from "@/components/ui/button";
import { SUPPORTED_MODELS, SUPPORTED_PROVIDERS } from "@/constants/evaluation";
import { CLIENT_EVENTS } from "@/constants/events";
import getMessage from "@/constants/messages";
import { EvaluationConfig } from "@/types/evaluation";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { usePostHog } from "posthog-js/react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

const EvaluationVaultCreate = ({
	successCallback,
}: {
	successCallback: () => void;
}) => {
	return (
		<div className="flex flex-wrap items-center gap-1 w-full h-full text-stone-700 dark:text-stone-300 ml-3">
			Unable to find the vault key.
			<SecretForm successCallback={successCallback}>
				<Button variant="ghost" className="py-0 text-link hover:bg-transparent dark:hover:bg-transaparent h-auto px-0 text-xs text-primary">
					Create new!
				</Button>
			</SecretForm>
		</div>
	);
};

const EVALUATION_TOAST_ID = "evaluation-config";
const EVALUATION_CONFIG_FORM = "evaluation-config-form";

function ModifyEvaluationSettings({
	evaluation,
	onSuccess,
}: {
	evaluation: EvaluationConfig | null;
	onSuccess: () => void;
}) {
	const [provider, setProvider] = useState(evaluation?.provider || "");
	const [autoEvaluation, setAutoEvaluation] = useState<boolean>(
		evaluation?.auto || false
	);
	const [model, setModel] = useState(evaluation?.model || "");
	const [vaultKeys, setVaultKeys] = useState([]);
	const posthog = usePostHog();
	const { fireRequest, isLoading: isLoadingModify } = useFetchWrapper();
	const { fireRequest: getVaultKeys, isLoading: isLoadingVaultKeys } =
		useFetchWrapper();

	const modifyDetails: FormBuilderEvent = (event) => {
		event.preventDefault();
		const formElement = event.target as HTMLFormElement;

		if (formElement.name !== EVALUATION_CONFIG_FORM) {
			return;
		}

		const autoValue = formElement.auto instanceof HTMLInputElement ? formElement.auto.checked : formElement.auto instanceof RadioNodeList ? (Array.from(formElement.auto).find(item => ((item as any).checked)) as any)?.checked : false;

		const bodyObject = {
			...(evaluation || {}),
			provider: (formElement.provider as any)?.value,
			model: (formElement.model as any)?.value,
			vaultId: (formElement.vaultId as any)?.value,
			auto: autoValue,
			recurringTime: (formElement.recurringTime as any)?.value,
		};

		if (!bodyObject.provider || !bodyObject.model || !bodyObject.vaultId) {
			toast.error(getMessage().EVALUATION_CONFIG_INVALID, {
				id: EVALUATION_TOAST_ID,
			});
			return;
		}
		toast.loading(getMessage().EVALUATION_CONFIG_MODIFYING, {
			id: EVALUATION_TOAST_ID,
		});

		fireRequest({
			body: JSON.stringify(bodyObject),
			requestType: "POST",
			url: "/api/evaluation/config",
			responseDataKey: "data",
			successCb: () => {
				toast.success(
					evaluation?.id
						? getMessage().EVALUATION_UPDATED
						: getMessage().EVALUATION_CREATED,
					{
						id: EVALUATION_TOAST_ID,
					}
				);
				formElement.reset();
				onSuccess();
				posthog?.capture(
					evaluation?.id
						? CLIENT_EVENTS.EVALUATION_CONFIG_UPDATED_SUCCESS
						: CLIENT_EVENTS.EVALUATION_CONFIG_CREATED_SUCCESS
				);
			},
			failureCb: (err?: string) => {
				toast.error(err || getMessage().EVALUATION_CONFIG_UPDATING_FAILED, {
					id: EVALUATION_TOAST_ID,
				});
				posthog?.capture(
					evaluation?.id
						? CLIENT_EVENTS.EVALUATION_CONFIG_UPDATED_FAILURE
						: CLIENT_EVENTS.EVALUATION_CONFIG_CREATED_FAILURE
				);
			},
		});
	};

	const fetchVaultKeys = () => {
		getVaultKeys({
			requestType: "POST",
			url: "/api/vault/get",
			responseDataKey: "data",
			body: JSON.stringify({}),
			successCb: (res) => {
				setVaultKeys(
					res?.length
						? res.map((key: any) => ({ label: key.key, value: key.id }))
						: []
				);
			},
		});
	};

	useEffect(() => {
		fetchVaultKeys();
	}, []);

	const providerOptions = useMemo(() => {
		return SUPPORTED_PROVIDERS.map((provider) => {
			return {
				label: provider,
				value: provider,
			};
		});
	}, []);
	const modelOptions = useMemo(() => {
		return (
			SUPPORTED_MODELS[provider as keyof typeof SUPPORTED_MODELS] || []
		).map((model: string) => {
			return {
				label: model,
				value: model,
			};
		});
	}, [provider]);

	return (
		<FormBuilder
			cardClassName="py-4 px-6"
			formName={EVALUATION_CONFIG_FORM}
			fields={[
				{
					label: "Model Provider",
					inputKey: `${evaluation?.id}-provider`,
					fieldType: "SELECT",
					fieldTypeProps: {
						name: "provider",
						placeholder: "Select provider",
						defaultValue: provider || "",
						options: providerOptions,
						onChange: (value: string) => {
							setProvider(value);
							setModel("");
						},
					},
				},
				{
					label: "Model",
					inputKey: `${evaluation?.id}-model`,
					fieldType: "SELECT",
					fieldTypeProps: {
						name: "model",
						placeholder: provider ? "Select model" : "Select provider first",
						defaultValue: model || "",
						onChange: (value: string) => {
							setModel(value);
						},
						options: modelOptions,
					},
				},
				{
					label: "Provider API Key",
					fieldType: "SELECT",
					inputKey: `${evaluation?.id}-vaultId`,
					fieldTypeProps: {
						name: "vaultId",
						placeholder: "Select vault key",
						options: vaultKeys,
						defaultValue: evaluation?.vaultId || "",
					},
					description: (
						<EvaluationVaultCreate successCallback={fetchVaultKeys} />
					),
				},
				{
					label: "Auto Evaluation",
					fieldType: "SWITCH",
					inputKey: `${evaluation?.id}-auto`,
					fieldTypeProps: {
						name: "auto",
						defaultChecked: evaluation?.auto || false,
						onCheckedChange: (value: boolean) => {
							setAutoEvaluation(value);
						},
					},
				},
				{
					label: "Recurring Time",
					inputKey: `${evaluation?.id}-recurringTime`,
					fieldType: "INPUT",
					fieldTypeProps: {
						type: "text",
						name: "recurringTime",
						placeholder: "* * * * *",
						defaultValue: evaluation?.recurringTime || "",
						disabled: !autoEvaluation,
					},
				},
			]}
			heading={`${evaluation?.id ? "Update" : "Create"} evaluation settings`}
			isLoading={isLoadingModify || isLoadingVaultKeys}
			onSubmit={modifyDetails}
			submitButtonText={evaluation?.id ? "Update" : "Create"}
		/>
	);
}

export default function Evaluation() {
	const {
		fireRequest: getEvaluationConfig,
		data,
		isLoading,
	} = useFetchWrapper<EvaluationConfig>();

	const fetchEvaluationConfig = () => {
		getEvaluationConfig({
			requestType: "GET",
			url: "/api/evaluation/config",
			responseDataKey: "data",
		});
	};

	useEffect(() => {
		fetchEvaluationConfig();
	}, []);

	return (
		<div className="flex flex-1 h-full w-full relative">
			{!isLoading && (
				<ModifyEvaluationSettings
					evaluation={data}
					onSuccess={fetchEvaluationConfig}
				/>
			)}
		</div>
	);
}
