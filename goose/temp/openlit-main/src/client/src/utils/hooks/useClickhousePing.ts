import { useCallback, useEffect, useState } from "react";
import { getData } from "@/utils/api";
import { toast } from "sonner";

export default function useClickhousePing() {
	const [isSuccess, setIsSuccess] = useState<boolean>(false);
	const [isLoading, setIsLoading] = useState<boolean>(true);
	const [error, setError] = useState<string>("");

	const pingClickhouse = useCallback(async () => {
		setIsLoading(true);
		setError("");
		try {
			const response = await getData({
				url: "/api/clickhouse",
				method: "GET",
			});

			if (response.err) {
				setError(response.err);
				toast.error(response.err || "Clickhouse connection failed!", {
					id: "PING",
				});
			} else {
				setIsSuccess(true);
			}
		} catch (error) {
			const updatedError = (error as any).toString().replaceAll("Error:", "");
			setError(updatedError);
			toast.error(updatedError || "Clickhouse connection failed!", {
				id: "PING",
			});
		}

		setIsLoading(false);
	}, []);

	useEffect(() => {
		pingClickhouse();
	}, []);

	return { isSuccess, error, isLoading };
}
