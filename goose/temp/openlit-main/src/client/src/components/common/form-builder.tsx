import { noop } from "@/utils/noop";
import React, { FormEvent } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import FormField from "./form-field";
import { set } from "lodash";
import { FieldProps, FormBuilderEvent } from "@/types/form";

const FormBuilder = ({
	fields,
	heading,
	subHeading,
	subHeadingClass,
	isLoading = false,
	onSubmit,
	submitButtonText,
	isAllowedToSubmit = true,
	alignment = "horizontal",
	formName = "builder-form",
	cardClassName = ""
}: {
	fields: FieldProps[];
	heading?: string;
	subHeading?: string;
	subHeadingClass?: string;
	isLoading?: boolean;
	onSubmit: FormBuilderEvent;
	submitButtonText: string;
	isAllowedToSubmit?: boolean;
	alignment?: "horizontal" | "vertical";
	formName?: string;
	cardClassName?: string
}) => {
	const getFormData = (e: FormEvent) => {
		const formElement = e.target as HTMLFormElement;
		return fields.reduce((acc: any, field) => {
			if (
				field.fieldType === "INPUT" ||
				field.fieldType === "TEXTAREA" ||
				field.fieldType === "RADIOGROUP"
			) {
				if (
					field.fieldTypeProps.name &&
					formElement[field.fieldTypeProps.name]
				) {
					acc[field.fieldTypeProps.name] =
						formElement[field.fieldTypeProps.name].value;
				}
			} else if (field.fieldType === "TAGSINPUT") {
				if (
					field.fieldTypeProps.name &&
					formElement[field.fieldTypeProps.name]
				) {
					if (
						NodeList.prototype.isPrototypeOf(
							formElement[field.fieldTypeProps.name]
						)
					) {
						acc[field.fieldTypeProps.name] = [
							...formElement[field.fieldTypeProps.name],
						].map((i) => i.value);
					} else {
						acc[field.fieldTypeProps.name] = [
							formElement[field.fieldTypeProps.name].value,
						];
					}
				} else if (field.fieldTypeProps.name) {
					acc[field.fieldTypeProps.name] = [];
				}
			} else if (field.fieldType === "KEYVALUE" && field.fieldTypeProps.name) {
				formElement
					.querySelectorAll(`[name*=${field.fieldTypeProps.name}]`)
					.forEach((element) => {
						set(
							acc,
							element.getAttribute("name")!,
							element.getAttribute("value")
						);
					});
			}
			return acc;
		}, {});
	};

	return (
		<form
			className="flex flex-col w-full h-full"
			onSubmit={(e) => {
				e.preventDefault();
				if (isLoading || !isAllowedToSubmit) {
					return noop();
				}

				return onSubmit(e, getFormData(e));
			}}
			onKeyDown={(e) => !(e.key === "Enter")}
			name={formName}
		>
			<Card className={`w-full border-0 flex flex-col h-full shadow-none  ${cardClassName} bg-transparent dark:bg-transparent`}>
				{heading && (
					<CardHeader className="shrink-0 px-0 pt-0 pb-4">
						<CardTitle className="text-2xl text-stone-600 dark:text-stone-400">{heading}</CardTitle>
						{subHeading && (
							<CardTitle className={`text-sm text-stone-600 dark:text-stone-400 ${subHeadingClass}`}>
								{subHeading}
							</CardTitle>
						)}
					</CardHeader>
				)}
				<CardContent className="flex gap-4 flex-col overflow-hidden p-0">
					<div className="grid gap-6 relative flex-1 overflow-y-auto overflow-x-hidden scrollbar-hidden">
						{fields.map((field, index) => {
							field.fieldTypeProps.id = field.id || field.fieldTypeProps.name || field.inputKey || `field-${index}`;
							return (
								<FormField
									key={index}
									{...field}
									boundaryClass={
										alignment === "horizontal"
											? "grid grid-cols-3 flex-1 items-center"
											: "grid grid-cols-1 flex-1 items-center gap-2"
									}
								/>
							);
						})}
					</div>
					<div className="flex items-center justify-end w-full gap-3">
						{isAllowedToSubmit && (
							<Button
								type="submit"
								className={`${isLoading ? "animate-pulse" : ""}`}
							>
								{submitButtonText}
							</Button>
						)}
					</div>
				</CardContent>
			</Card>
		</form>
	);
};

export default FormBuilder;
