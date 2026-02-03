import FormBuilder from "@/components/common/form-builder";
import { FieldProps, FormBuilderEvent } from "@/types/form";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { CLIENT_EVENTS } from "@/constants/events";
import { PromptInput, PromptUpdate } from "@/constants/prompts";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { jsonParse } from "@/utils/json";
import { objectEntries } from "@/utils/object";
import { unescapeString } from "@/utils/string";
import { useRouter } from "next/navigation";
import { usePostHog } from "posthog-js/react";
import { KeyboardEvent, useCallback, useState } from "react";
import { toast } from "sonner";

const getVersions = (startingVersion: string) => {
	const versionNumbers = startingVersion
		.split(".")
		.map((val) => parseInt(val, 10));
	return {
		draft: startingVersion,
		major: [versionNumbers[0] + 1, 0, 0].join("."),
		minor: [versionNumbers[0], versionNumbers[1] + 1, 0].join("."),
		patch: [versionNumbers[0], versionNumbers[1], versionNumbers[2] + 1].join(
			"."
		),
	};
};

const DEFAULT_META_PROPERTIES = {
	model: "",
	temperature: "",
};

export default function PromptForm({
	versionData,
	children,
	successCallback,
}: {
	versionData?: any;
	children: JSX.Element;
	successCallback?: () => void;
}) {
	const posthog = usePostHog();
	const router = useRouter();
	const [isOpen, setIsOpen] = useState(false);
	const { fireRequest, isLoading } = useFetchWrapper();
	const { fireRequest: fireUpdateRequest, isLoading: isUpdateLoading } =
		useFetchWrapper();
	const createPrompt: FormBuilderEvent = useCallback((event, formdata) => {
		event.preventDefault();
		toast.loading("Creating prompt...", {
			id: "prompt-hub",
		});

		const payload: PromptInput = formdata;

		payload.metaProperties = (formdata.metaProperties || []).reduce(
			(
				acc: Record<string, unknown>,
				{ key, value }: { key: string; value: unknown }
			) => {
				acc[key] = value;
				return acc;
			},
			{}
		);

		payload.status = payload.version === "0.0.0" ? "DRAFT" : "PUBLISHED";

		fireRequest({
			body: JSON.stringify(payload),
			requestType: "POST",
			url: "/api/prompt",
			successCb: (response: any) => {
				toast.success("Created prompt successfully!", {
					id: "prompt-hub",
				});
				setIsOpen(false);
				posthog?.capture(CLIENT_EVENTS.PROMPT_ADD_SUCCESS);
				if (response?.data?.promptId) {
					router.push(`/prompt-hub/${response?.data?.promptId}`);
				}
				if (typeof successCallback === "function") {
					successCallback();
				}
			},
			failureCb: (err?: string) => {
				toast.error(err || "Creation of prompt failed!", {
					id: "prompt-hub",
				});
				posthog?.capture(CLIENT_EVENTS.PROMPT_ADD_FAILURE);
			},
		});
	}, []);

	const updateVersion: FormBuilderEvent = useCallback((event, formdata) => {
		event.preventDefault();
		toast.loading("Update prompt version...", {
			id: "prompt-hub",
		});

		const payload: PromptUpdate = formdata;

		payload.metaProperties = (formdata.metaProperties || {}).reduce(
			(
				acc: Record<string, unknown>,
				{ key, value }: { key: string; value: unknown }
			) => {
				acc[key] = value;
				return acc;
			},
			{}
		);

		payload.status =
			payload.version === versionData.version ? "DRAFT" : "PUBLISHED";

		if (versionData.status === "DRAFT") {
			payload.versionId = versionData.versionId;
		}

		payload.promptId = versionData.promptId;

		fireUpdateRequest({
			body: JSON.stringify(payload),
			requestType: "POST",
			url: "/api/prompt/version",
			successCb: (response: any) => {
				toast.success("Updated prompt version successfully!", {
					id: "prompt-hub",
				});
				setIsOpen(false);
				posthog?.capture(CLIENT_EVENTS.PROMPT_VERSION_ADD_SUCCESS);
				if (response?.data?.promptId) {
					router.push(
						`/prompt-hub/${response?.data?.promptId}?version=${payload.version}`
					);
				}
				if (typeof successCallback === "function") {
					successCallback();
				}
			},
			failureCb: (err?: string) => {
				toast.error(err || "Updation of prompt version failed!", {
					id: "prompt-hub",
				});
				posthog?.capture(CLIENT_EVENTS.PROMPT_VERSION_ADD_FAILURE);
			},
		});
	}, []);

	let formFields: FieldProps[] = [];
	const defaultTags = versionData?.tags ? jsonParse(versionData?.tags) : [];
	let defaultMetaProperties = versionData?.metaProperties
		? jsonParse(versionData?.metaProperties)
		: DEFAULT_META_PROPERTIES;
	defaultMetaProperties = objectEntries(defaultMetaProperties).reduce(
		(acc: any, [key, value]) => {
			acc.push({ key, value });
			return acc;
		},
		[]
	);

	const newVersions = getVersions(versionData?.version || "0.0.0");

	if (!versionData?.versionId) {
		formFields.push({
			label: "Name",
			description: "Only small alphabets and _ are allowed",
			inputKey: `${versionData?.versionId || "new"}-name`,
			fieldType: "INPUT",
			fieldTypeProps: {
				type: "text",
				name: "name",
				placeholder: "",
				defaultValue: "",
				onKeyUp: (event: KeyboardEvent) => {
					const target = event.target as HTMLInputElement;
					let input = target.value;

					// Replace spaces with underscores
					input = input.toLowerCase().replace(/ /g, "_");

					// Allow only lowercase alphabet letters and underscores
					input = input.replace(/[^a-z_]/g, "");

					// Update the input field value
					target.value = input;
				},
			},
		});
	}
	formFields = formFields.concat([
		{
			label: "Prompt",
			description:
				"You can use {{variableName}} in the prompt to create dynamic prompts.",
			inputKey: `${versionData?.versionId || "new"}-prompt`,
			fieldType: "TEXTAREA",
			fieldTypeProps: {
				name: "prompt",
				placeholder: "How can I assist you?",
				defaultValue: unescapeString(versionData?.prompt || ""),
			},
		},
		{
			label: "Tags",
			inputKey: `${versionData?.versionId || "new"}-tags`,
			fieldType: "TAGSINPUT",
			fieldTypeProps: {
				name: "tags",
				placeholder: "Add tags",
				defaultValue: defaultTags,
			},
		},
		{
			label: "Meta Properties",
			inputKey: `${versionData?.versionId || "new"}-metaProperties`,
			fieldType: "KEYVALUE",
			fieldTypeProps: {
				name: "metaProperties",
				placeholder: "Meta",
				defaultValue: defaultMetaProperties,
			},
		},
		{
			label: "Version",
			inputKey: `${versionData?.versionId || "new"}-version`,
			fieldType: "RADIOGROUP",
			fieldTypeProps: {
				name: "version",
				placeholder: "Select version",
				defaultValue: newVersions.draft,
				options: [
					{
						title: "Draft state",
						subText: "No Version change",
						value: newVersions.draft,
						description: "This will be in draft state",
					},
					{
						title: "Major",
						subText: `Version : ${newVersions.major}`,
						value: newVersions.major,
						description:
							"Significant changes that are not backwards compatible",
					},
					{
						title: "Minor",
						subText: `Version : ${newVersions.minor}`,
						value: newVersions.minor,
						description: "New features that are backwards compatible",
					},
					{
						title: "Patch",
						subText: `Version : ${newVersions.patch}`,
						value: newVersions.patch,
						description: "Bug fixes and minor updates",
					},
				],
			},
		},
	]);

	return (
		<Dialog onOpenChange={setIsOpen} open={isOpen}>
			<DialogTrigger asChild>{children}</DialogTrigger>
			<DialogContent className="max-w-4xl h-[80%]">
				<DialogHeader>
					<DialogTitle className="dark:text-stone-200 text-stone-800">
						{versionData?.versionId
							? "Create new / Update current version of prompt"
							: "Create a new prompt"}
					</DialogTitle>
					<DialogDescription>
						You can create a new version of prompt in Draft/Published mode
					</DialogDescription>
				</DialogHeader>
				<div className="flex items-center overflow-y-auto">
					<FormBuilder
						alignment="vertical"
						fields={formFields}
						isLoading={isLoading || isUpdateLoading}
						onSubmit={versionData?.versionId ? updateVersion : createPrompt}
						submitButtonText={`${
							versionData?.versionId
								? "Create new version"
								: "Create new prompt"
						}`}
					/>
				</div>
			</DialogContent>
		</Dialog>
	);
}
