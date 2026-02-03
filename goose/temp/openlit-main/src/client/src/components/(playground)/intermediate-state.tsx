import type { ReactNode } from "react";

export type IntermediateState = {
	type: "loading" | "nodata";
	children?: ReactNode;
	classNames?: string;
};
export default function IntermediateState(props: IntermediateState) {
	if (props.type === "loading")
		return props.children ? (
			props.children
		) : (
			<div
				className={`flex w-full items-center justify-center p-3 text-sm  text-stone-500 ${
					props.classNames || ""
				}`}
			>
				Loading...
			</div>
		);
	if (props.type === "nodata")
		return (
			<div
				className={`flex w-full items-center justify-center p-3 text-sm  text-stone-500  ${
					props.classNames || ""
				}`}
			>
				No data to display
			</div>
		);

	return null;
}
