import Search from "./search";

export default function Header({ title, children }: { title: string, children?: React.ReactNode }) {
	return <div className="flex justify-between items-center text-stone-700 dark:text-stone-300 gap-2">
		<h3 className="font-medium">{title}</h3>
		<div className="flex gap-2">
			<Search />
			{children}
		</div>
	</div>;
}