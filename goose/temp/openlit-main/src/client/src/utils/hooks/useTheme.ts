import { get, set } from "tiny-cookie";
import { useCallback, useEffect, useState } from "react";

type THEME = "dark" | "light";

export default function useTheme() {
	const [theme, setTheme] = useState<THEME>("light");
	const toggleTheme = useCallback((themeParam?: THEME) => {
		const value: THEME = themeParam || (theme === "dark" ? "light" : "dark");
		document.documentElement.classList.remove(theme);
		document.documentElement.classList.add(value);
		set("theme", value);
		setTheme(value);
	}, [theme]);

	useEffect(() => {
		const currentTheme = get("theme") as unknown as THEME;
		setTheme(currentTheme);
	});

	return { toggleTheme, theme };
}
