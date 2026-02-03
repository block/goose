"use client";
import { pingActiveDatabaseConfig } from "@/helpers/client/database-config";
import { getPingDetails } from "@/selectors/database-config";
import { useRootStore } from "@/store";
import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect } from "react";
import Loader from "../common/loader";

const ALLOWED_CONNECTIVITY_ALERT = /^\/home$|^\/dashboard$|^\/requests$|^\/exceptions$|^\/d\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$|^\/prompt-hub$|^\/vault$|^\/dashboards/;

export default function ClickhouseConnectivityWrapper({ children }: { children: React.ReactNode }) {
	const pingDetails = useRootStore(getPingDetails);
	const pathname = usePathname();

	useEffect(() => {
		if (pingDetails.status === "pending") pingActiveDatabaseConfig();
	}, []);

	if (!ALLOWED_CONNECTIVITY_ALERT.test(pathname)) {
		return children;
	}

	if (pingDetails.error) {
		return (
			<div className="p-4 mb-4 text-red-800 border border-red-300 rounded-md bg-red-50 dark:bg-red-950 dark:text-red-400 dark:border-red-800 w-full">
				<div className="flex">
					<div className="flex flex-col grow">
						<h3 className="text-lg font-medium">
							Looks like you&apos;ve found the doorway to the great nothing
						</h3>
						<div className="mb-2 text-sm">
							Sorry about that! Please visit settings page to configure your
							active clickhouse database.
						</div>
						<div className="mb-2 text-sm text-red-500">
							{pingDetails.error}
						</div>
						<Link
							href="/settings/database-config"
							className="inline-flex my-2 border rounded md py-2 px-4 text-center bg-primary cursor-pointer text-white hover:bg-stone-950 outline-none self-start text-sm"
						>
							Take me there!
						</Link>
					</div>
					<Image
						alt="Connection failure"
						src="/images/connect.svg"
						className="flex-shrink-0 w-24 h-24 me-2 self-center"
						height="128"
						width="128"
					/>
				</div>
			</div>
		);
	}

	if (pingDetails.status === "pending") {
		return (
			<div className="flex items-center justify-center h-full w-full">
				<Loader />
			</div>
		);
	}

	if (pingDetails.status === "success") {
		return children;
	}

	return null;
}
