import FormBuilder from "@/components/common/form-builder";
import { FieldProps, FormBuilderEvent } from "@/types/form";
import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { CLIENT_EVENTS } from "@/constants/events";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { jsonParse } from "@/utils/json";
import { isEmpty } from "lodash";
import { usePostHog } from "posthog-js/react";
import { KeyboardEvent, useCallback, useState } from "react";
import { toast } from "sonner";

export default function SecretForm({
	secretData,
	children,
	successCallback,
}: {
	secretData?: any;
	children: JSX.Element;
	successCallback?: () => void;
}) {
	const posthog = usePostHog();
	const [isOpen, setIsOpen] = useState(false);
	const { fireRequest, isLoading } = useFetchWrapper();
	const upsertSecret: FormBuilderEvent = useCallback((event, formdata) => {
		event.preventDefault();
		toast.loading("Creating secret...", {
			id: "vault",
		});

		const payload: any = formdata;
		if (isEmpty(payload.value)) {
			delete payload.value;
		}
		payload.id = secretData?.id;

		fireRequest({
			body: JSON.stringify(payload),
			requestType: payload.id ? "PUT" : "POST",
			url: "/api/vault",
			successCb: () => {
				toast.success(
					`${payload.id ? "Updated" : "Created"} secret successfully!`,
					{
						id: "vault",
					}
				);
				setIsOpen(false);
				posthog?.capture(
					payload.id
						? CLIENT_EVENTS.VAULT_SECRET_UPDATE_SUCCESS
						: CLIENT_EVENTS.VAULT_SECRET_ADD_SUCCESS
				);
				if (typeof successCallback === "function") {
					successCallback();
				}
			},
			failureCb: (err?: string) => {
				toast.error(
					err || `${payload.id ? "Updation" : "Creation"} of secret failed!`,
					{
						id: "vault",
					}
				);
				posthog?.capture(
					payload.id
						? CLIENT_EVENTS.VAULT_SECRET_UPDATE_SUCCESS
						: CLIENT_EVENTS.VAULT_SECRET_ADD_SUCCESS
				);
			},
		});
	}, []);

	const defaultTags = secretData?.tags ? jsonParse(secretData?.tags) : [];
	const formFields: FieldProps[] = [
		{
			label: "Key",
			description: `Only capital alphabets, digits and _ are allowed. ${
				secretData?.id ? "Cannot update the key" : ""
			}`,
			inputKey: `${secretData?.id || "new"}-key`,
			fieldType: "INPUT",
			fieldTypeProps: {
				type: "text",
				name: "key",
				placeholder: "YOUR_SECRET_KEY",
				defaultValue: secretData?.key || "",
				readOnly: !!secretData?.key,
				disabled: !!secretData?.key,
				onKeyUp: (event: KeyboardEvent) => {
					const target = event.target as HTMLInputElement;
					let input = target.value;

					// Replace spaces with underscores
					input = input.toUpperCase().replace(/ /g, "_");

					// Allow only uppercase alphabet letters and underscores
					input = input.replace(/[^A-Z0-9_]/g, "");

					// Update the input field value
					target.value = input;
				},
			},
		},
		{
			label: "Value",
			description: "Add your secret values",
			inputKey: `${secretData?.id || "new"}-value`,
			fieldType: "INPUT",
			fieldTypeProps: {
				type: "password",
				name: "value",
				placeholder: "*****",
				defaultValue: secretData?.value,
			},
		},
		{
			label: "Tags",
			inputKey: `${secretData?.versionId || "new"}-tags`,
			fieldType: "TAGSINPUT",
			fieldTypeProps: {
				name: "tags",
				placeholder: "Add tags",
				defaultValue: defaultTags,
			},
		},
	];

	return (
		<Dialog onOpenChange={setIsOpen} open={isOpen}>
			<DialogTrigger asChild>{children}</DialogTrigger>
			<DialogContent className="max-w-xl">
				<DialogHeader>
					<DialogTitle className="dark:text-stone-200 text-stone-800">
						{secretData?.id ? "Update a secret" : "Create a new secret"}
					</DialogTitle>
				</DialogHeader>
				<div className="flex items-center overflow-y-auto">
					<FormBuilder
						alignment="vertical"
						fields={formFields}
						isLoading={isLoading}
						onSubmit={upsertSecret}
						submitButtonText={`${
							secretData?.id ? "Update secret" : "Create secret"
						}`}
					/>
				</div>
			</DialogContent>
		</Dialog>
	);
}
