import { Button } from "@/components/ui/button";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { noop } from "@/utils/noop";
import copy from "copy-to-clipboard";
import { toast } from "sonner";
import FormBuilder from "@/components/common/form-builder";
import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { jsonStringify } from "@/utils/json";
import { useState } from "react";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";
import { FieldProps, FormBuilderEvent } from "@/types/form";

export default function Generate({ refresh }: { refresh: () => void }) {
	const posthog = usePostHog();
	const [isOpen, setIsOpen] = useState(false);
	const { fireRequest: fireCreateRequest, isLoading: isCreating } =
		useFetchWrapper();
	const handleCreation: FormBuilderEvent = async (event, formdata) => {
		event.preventDefault();
		if (formdata.name?.length < 3) {
			toast.error("Name length should be greater than 2...", {
				id: "api-key",
			});
			return;
		}
		toast.loading("Generating api key...", {
			id: "api-key",
		});
		fireCreateRequest({
			requestType: "POST",
			url: `/api/api-key`,
			body: jsonStringify({
				name: formdata.name,
			}),
			successCb: (data: any) => {
				copy(data.apiKey);
				toast.success("Generated and Copied new API key!", {
					id: "api-key",
				});
				setIsOpen(false);
				refresh();
				posthog?.capture(CLIENT_EVENTS.API_KEY_ADD_SUCCESS);
			},
			failureCb: (err?: string) => {
				toast.error(err || `Cannot connect to server!`, {
					id: "api-key",
				});
				posthog?.capture(CLIENT_EVENTS.API_KEY_ADD_FAILURE);
			},
		});
	};

	const formFields: FieldProps[] = [
		{
			label: "Name",
			description: "Assign a name to api key for better references in future",
			inputKey: `api-name`,
			fieldType: "INPUT",
			fieldTypeProps: {
				type: "text",
				name: "name",
				placeholder: "",
				defaultValue: "default",
			},
		},
	];

	return (
		<Dialog onOpenChange={setIsOpen} open={isOpen}>
			<DialogTrigger asChild>
				<Button
					variant="secondary"
					className="bg-primary hover:bg-primary dark:bg-primary dark:hover:bg-primary text-stone-100 dark:text-stone-100 px-8 py-1 h-8 self-end"
				>
					Generate New API Key
				</Button>
			</DialogTrigger>
			<DialogContent className="max-w-lg">
				<DialogHeader>
					<DialogTitle className="dark:text-stone-200 text-stone-800">
						Create a new api key
					</DialogTitle>
				</DialogHeader>
				<div className="flex items-center overflow-y-auto">
					<FormBuilder
						alignment="vertical"
						fields={formFields}
						isLoading={isCreating}
						onSubmit={isCreating ? noop : handleCreation}
						submitButtonText="Create"
					/>
				</div>
			</DialogContent>
		</Dialog>
	);
}
