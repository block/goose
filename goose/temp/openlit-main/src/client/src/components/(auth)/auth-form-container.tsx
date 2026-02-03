"use client";
import { MoveRightIcon } from "lucide-react";
import { GithubIcon } from "lucide-react";
import { Button } from "../ui/button";
import Link from "next/link";
import Confetti from "../common/confetti";

export default function AuthFormContainer({
	children,
}: {
	children: JSX.Element;
}) {
	return (
		<div className="flex flex-col justify-center p-8 lg:p-16 bg-stone-50 relative">
			<Confetti />
			<div className="flex flex-col w-full max-w-sm mx-auto gap-12 z-10">
				<div className="text-center">
					<h1 className="text-4xl font-bold tracking-tight text-primary">
						Welcome to OpenLIT
					</h1>
					<p className="text-stone-600">
						Open Source Platform for AI Engineering
					</p>
				</div>
				{children}
				<div className="grid grid-cols-2 text-center text-sm">
					<Link
						href={"https://github.com/openlit/openlit"}
						target="_blank"
						className="w-full"
					>
						<Button
							className={`w-full rounded-full gap-2 font-bold bg-stone-900 text-stone-50 hover:bg-stone-900/90 dark:bg-stone-900 dark:text-stone-50 dark:hover:bg-stone-900/90`}
						>
							Github
							<GithubIcon className="ml-2 w-4" />
						</Button>
					</Link>
					<Link
						href="https://docs.openlit.io/latest/overview"
						target="_blank"
					>
						<Button variant={"ghost"} className="hover:bg-stone-100 hover:text-stone-900 dark:hover:bg-stone-100 dark:hover:text-stone-900">
							<b>Documentation</b>
							<MoveRightIcon className="ml-2 h-5 w-5" />
						</Button>
					</Link>
				</div>
			</div>
		</div>
	);
}
