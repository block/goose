import { ReactNode } from "react"

export default function FeatureHero({
	iconComponent,
	title,
	description,
}: {
	iconComponent: ReactNode
	title: string,
	description: string,
}) {
	return (
		<div className="border dark:border-stone-800 rounded-lg p-6">
			<div className="flex items-start space-x-4">
				<div className="flex-shrink-0">
					<div className="w-10 h-10 bg-stone-200 dark:bg-stone-700 rounded-full p-1.5 flex items-center justify-center">
						{iconComponent}
					</div>
				</div>
				<div className="flex-1">
					<h2 className="text-xl font-semibold text-stone-600 dark:text-stone-300 mb-2">
						{title}
					</h2>
					<p className="text-stone-600 dark:text-stone-300">
						{description}
					</p>
				</div>
			</div>
		</div>
	)
}