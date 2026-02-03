"use client";

import FormBuilder from "@/components/common/form-builder";
import { FormBuilderEvent } from "@/types/form";
import { CLIENT_EVENTS } from "@/constants/events";
import { getUserDetails, setUser } from "@/selectors/user";
import { useRootStore } from "@/store";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { User } from "@prisma/client";
import { usePostHog } from "posthog-js/react";
import { toast } from "sonner";
import { PRIMARY_BACKGROUND } from "@/constants/common-classes";

const PROFILE_TOAST_ID = "profile-details";

function ModifyProfileDetails({
	user,
	fetchUser,
}: {
	user: User | null;
	fetchUser: () => void;
}) {
	const posthog = usePostHog();
	const { fireRequest, isLoading } = useFetchWrapper();

	const modifyDetails: FormBuilderEvent = (event) => {
		event.preventDefault();
		const formElement = event.target as HTMLFormElement;

		const bodyObject = {
			currentPassword: (formElement.currentPassword as any)?.value,
			newPassword: (formElement.newPassword as any)?.value,
			name: (formElement.name as any)?.value,
		};

		if (
			bodyObject.newPassword !== (formElement.confirmNewPassword as any)?.value
		) {
			toast.loading("New password and Confirm new password does not match...", {
				id: PROFILE_TOAST_ID,
			});
			return;
		}

		toast.loading("Modifying profile details...", {
			id: PROFILE_TOAST_ID,
		});

		fireRequest({
			body: JSON.stringify(bodyObject),
			requestType: "POST",
			url: "/api/user/profile",
			responseDataKey: "data",
			successCb: () => {
				toast.success("Profile details updated!", {
					id: PROFILE_TOAST_ID,
				});
				formElement.reset();
				fetchUser();
				posthog?.capture(CLIENT_EVENTS.PROFILE_UPDATE_SUCCESS);
			},
			failureCb: (err?: string) => {
				toast.error(err || "Profile details updation failed!", {
					id: PROFILE_TOAST_ID,
				});
				posthog?.capture(CLIENT_EVENTS.PROFILE_UPDATE_FAILURE);
			},
		});
	};

	return (
		<FormBuilder
			cardClassName={`${PRIMARY_BACKGROUND} py-4 px-6`}
			fields={[
				{
					label: "Email",
					inputKey: `${user?.email}-email`,
					fieldType: "INPUT",
					fieldTypeProps: {
						type: "text",
						name: "email",
						placeholder: "db-config",
						defaultValue: user?.email || "",
						disabled: true,
					},
				},
				{
					label: "Profile Name",
					inputKey: `${user?.id}-name`,
					fieldType: "INPUT",
					fieldTypeProps: {
						type: "text",
						name: "name",
						placeholder: "db-config",
						defaultValue: user?.name || "",
					},
				},
				{
					label: "Current Password",
					inputKey: `${user?.id}-currentPassword`,
					fieldType: "INPUT",
					fieldTypeProps: {
						type: "password",
						name: "currentPassword",
						placeholder: "*******",
					},
				},
				{
					label: "New Password",
					fieldType: "INPUT",
					inputKey: `${user?.id}-newPassword`,
					fieldTypeProps: {
						type: "password",
						name: "newPassword",
						placeholder: "*******",
					},
				},
				{
					label: "Confirm New Password",
					fieldType: "INPUT",
					inputKey: `${user?.id}-confirmNewPassword`,
					fieldTypeProps: {
						type: "password",
						name: "confirmNewPassword",
						placeholder: "*******",
					},
				},
			]}
			heading={`Update profile details`}
			isLoading={isLoading}
			onSubmit={modifyDetails}
			submitButtonText={"Update"}
		/>
	);
}

export default function Profile() {
	const userDetails = useRootStore(getUserDetails);
	const setUserDetails = useRootStore(setUser);
	const { fireRequest: getUser } = useFetchWrapper();

	const fetchUser = () => {
		getUser({
			requestType: "GET",
			url: "/api/user/profile",
			responseDataKey: "data",
			successCb(res) {
				setUserDetails(res);
			},
			failureCb: (err?: string) => {
				toast.error(err || "Unauthorized access!", {
					id: PROFILE_TOAST_ID,
				});
			},
		});
	};

	return (
		<div className="flex flex-1 h-full w-full relative">
			<ModifyProfileDetails user={userDetails as User} fetchUser={fetchUser} />
		</div>
	);
}
