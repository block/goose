import { Suspense } from "react";
import { AuthForm } from "@/components/(auth)/auth-form";

export default function Register() {
	return (
		<Suspense fallback={null}>
			<AuthForm type={"register"} />
		</Suspense>
	);
}
