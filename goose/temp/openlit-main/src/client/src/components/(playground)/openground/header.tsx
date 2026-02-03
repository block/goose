import { Button } from "@/components/ui/button";
import { getEvaluatedResponse, resetOpenground } from "@/selectors/openground";
import { useRootStore } from "@/store";
import Link from "next/link";
import { HeaderPortal } from "../header-portal";
import { SettingsIcon } from "lucide-react";
import getMessage from "@/constants/messages";

export default function OpengroundHeader({
	className = "flex w-full items-center justify-end gap-2",
	validateResponse = true,
}: {
	className?: string;
	validateResponse: boolean;
}) {
	const evaluatedResponse = useRootStore(getEvaluatedResponse);
	const resetOpengroundData = useRootStore(resetOpenground);

	const showButton =
		(validateResponse && !!evaluatedResponse.data) || !validateResponse;

	return (
		<HeaderPortal>
			<div className={className}>
				{showButton ? (
					<>
						<Link href={"/openground/models"}>
							<Button variant="outline" className="h-auto py-1">
								<SettingsIcon className="h-4 w-4 mr-2" />
								{getMessage().OPENGROUND_MANAGE_MODELS}
							</Button>
						</Link>
						<Link href={"/openground/new"} onClick={resetOpengroundData}>
							<Button variant="secondary" className="bg-primary hover:bg-primary dark:bg-primary dark:hover:bg-primary text-stone-100 dark:text-stone-100 h-auto py-1">{getMessage().OPENGROUND_CREATE_NEW_PLAYGROUND}</Button>
						</Link>
					</>
				) : null}
			</div>
		</HeaderPortal>
	);
}
