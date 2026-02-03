import useTheme from "@/utils/hooks/useTheme";
import { Button } from "@/components/ui/button";
import { Moon, Sun } from "lucide-react";


export default function ThemeToggleSwitch() {
	const { toggleTheme } = useTheme();
	const handleDark = () => toggleTheme("dark");
	const handleLight = () => toggleTheme("light");

	return (
		<div className="flex items-center rounded-full relative bg-stone-200 dark:bg-stone-800">
			<div className="h-full aspect-square rounded-full bg-primary absolute top-0 left-0 z-0 dark:left-unset dark:right-0 dark:translate-x-full transition-all border-stone-200 dark:border-stone-800 border-[3px] box-border" />
			<Button
				variant="ghost"
				size={"icon"}
				className="rounded-full w-8 h-8 z-[1] p-2 hover:bg-transparent dark:hover:bg-transparent hover:text-stone-50 dark:hover:text-stone-300 text-stone-50 dark:text-stone-300"
				onClick={handleLight}
			>
				<Sun className="size-5" />
			</Button>
			<Button
				variant="ghost"
				size={"icon"}
				className="rounded-full w-8 h-8 z-[1] p-2 hover:bg-transparent dark:hover:bg-transparent text-stone-500 hover:text-stone-500 dark:text-stone-100"
				onClick={handleDark}
			>
				<Moon className="size-5" />
			</Button>
		</div>
	);
};