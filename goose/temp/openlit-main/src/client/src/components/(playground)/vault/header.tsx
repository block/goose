import { Button } from "@/components/ui/button";
import SecretForm from "./form";
import { useRootStore } from "@/store";
import { getPingStatus } from "@/selectors/database-config";
import VaultUsage from "./usage";

export default function VaultHeader({
	className = "flex w-full items-center justify-end gap-4",
	successCallback,
}: {
	className?: string;
	successCallback?: () => void;
}) {
	const pingStatus = useRootStore(getPingStatus);

	return (
		<div className={className}>
			{pingStatus === "success" && <VaultUsage />}
			{pingStatus === "success" && (
				<SecretForm successCallback={successCallback}>
					<Button
						variant="secondary"
						className="bg-primary hover:bg-primary dark:bg-primary dark:hover:bg-primary text-stone-100 dark:text-stone-100 px-8 h-9 py-0.5"
					>
						Create new
					</Button>
				</SecretForm>
			)}
		</div>
	);
}
