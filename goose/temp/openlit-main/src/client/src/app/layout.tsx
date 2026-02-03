import type { Metadata } from "next";
import { cookies } from "next/headers";
import localFont from "next/font/local";
import "./globals.css";
import { Toaster } from "@/components/ui/sonner";

const inter = localFont({
	src: "../../public/fonts/Inter.ttf",
	variable: "--font-inter",
	weight: "100 900",
	display: "swap",
});

export const metadata: Metadata = {
	title: "OpenLIT | Open Source Observability for LLMs",
	description:
		"Open-source tool for tracking and analyzing usage patterns of Large Language Models (LLMs).",
};

export default function RootLayout({
	children,
}: {
	children: React.ReactNode;
}) {
	const cookieStore = cookies();
	const theme = cookieStore.get("theme");

	return (
		<html lang="en" className={`scroll-smooth ${theme?.value || ""}`}>
			{/* bg-[linear-gradient(360deg,rgba(243,108,6,0.7)_0%,white_25%)] dark:bg-[linear-gradient(360deg,rgba(243,108,6,0.8)_0%,black_25%)] */}
			<body className={`${inter.className} bg-stone-50 dark:bg-stone-950`}>
				{children}
				<Toaster position="bottom-right" />
			</body>
		</html>
	);
}
