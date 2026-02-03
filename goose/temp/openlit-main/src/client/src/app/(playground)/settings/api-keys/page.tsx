"use client";
import ManageKeys from "@/components/(playground)/api-keys/manage";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Key, Shield, RotateCcw, Trash2 } from "lucide-react";

export default function APIKeys() {
	return (
		<div className="flex w-full flex-1 overflow-hidden">
			<div className="flex flex-col grow w-full rounded overflow-hidden text-sm text-stone-900 dark:text-stone-300 gap-6 p-4">
				{/* API Keys Information Callout */}
				<Alert className="border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-950/20 text-stone-700 dark:text-stone-300">
					<Key className="h-5 w-5 stroke-stone-700 dark:stroke-stone-300" />
					<AlertTitle className="text-base font-semibold">
						API Key Management
					</AlertTitle>
					<AlertDescription className="mt-2">
						<p className="mb-4">
							Welcome to the API Key Management page. Here, you can view, generate,
							and manage API keys for seamless integration with our services. Please
							note that we do not display your secret API keys again after you
							generate them.
						</p>
						
						<div className="space-y-3">
							<div className="flex items-start gap-3">
								<Shield className="h-4 w-4 mt-0.5 flex-shrink-0" />
								<div>
									<span className="font-semibold">Keep Your Keys Secure:</span>
									<span className="ml-1">Treat your API keys like passwords. Do not share them publicly or expose them in places where unauthorized individuals may access them.</span>
								</div>
							</div>
							
							<div className="flex items-start gap-3">
								<RotateCcw className="h-4 w-4 mt-0.5 flex-shrink-0" />
								<div>
									<span className="font-semibold">Rotate Keys Regularly:</span>
									<span className="ml-1">For enhanced security, consider rotating your keys periodically.</span>
								</div>
							</div>
							
							<div className="flex items-start gap-3">
								<Trash2 className="h-4 w-4 mt-0.5 flex-shrink-0" />
								<div>
									<span className="font-semibold">Revoke Unused Keys:</span>
									<span className="ml-1">If a key is no longer needed or compromised, revoke it immediately.</span>
								</div>
							</div>
						</div>
					</AlertDescription>
				</Alert>
				<ManageKeys />
			</div>
		</div>
	);
}
