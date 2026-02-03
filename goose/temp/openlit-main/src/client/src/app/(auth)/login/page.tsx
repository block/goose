import { Suspense } from "react";
import { AuthForm } from "@/components/(auth)/auth-form";

export default function Login() {
	return (
		<Suspense fallback={null}>
			<AuthForm type={"login"} />
		</Suspense>
	);
}
