"use client";
import { DEFAULT_LOGGED_IN_ROUTE } from "@/constants/route";
import asaw from "@/utils/asaw";
import { getProviders, signIn } from "next-auth/react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { usePostHog } from "posthog-js/react";
import { CLIENT_EVENTS } from "@/constants/events";

const errors = {
	AccessDenied: "Access denied for this account.",
	Signin: "Try signing with a different account.",
	OAuthSignin: "Try signing with a different account.",
	OAuthCallback: "Try signing with a different account.",
	OAuthCreateAccount: "Try signing with a different account.",
	EmailCreateAccount: "Try signing with a different account.",
	Callback: "Try signing with a different account.",
	OAuthAccountNotLinked:
		"To confirm your identity, sign in with the same account you used originally.",
	EmailSignin: "Check your email address.",
	CredentialsSignin:
		"Sign in failed. Check the details you provided are correct.",
	Configuration: "There is a problem with the server configuration.",
	default: "Unable to sign in.",
};

const SignInError = ({ error }: { error: string }) => {
	if (!error || error === "undefined") return null;

	const errorMessage = errors[error as keyof typeof errors] ?? error ?? errors.default;
	return <div className="text-xs text-red-600 text-center bg-red-50 p-3 rounded-md">{errorMessage}</div>;
};

export function AuthForm({ type }: { type: "login" | "register" }) {
	const posthog = usePostHog();
	const searchParams = useSearchParams();
	const callbackUrl: string =
		(searchParams.get("callbackUrl") as string) || DEFAULT_LOGGED_IN_ROUTE;
	const errorType: string = (searchParams.get("error") as string);
	const [error, setError] = useState<string>(errorType || "");
	const [isGoogleLoading, setIsGoogleLoading] = useState(false);
	const [isGithubLoading, setIsGithubLoading] = useState(false);
	const [providers, setProviders] = useState<string[]>([]);

	useEffect(() => {
		const fetchProviders = async () => {
			const res = await getProviders();
			if (typeof res === "object" && res !== null) {
				setProviders(Object.keys(res).filter(e => !["login", "register"].includes(e)));
			}
		};
		fetchProviders();
	}, []);

	async function authWrapper(fn: any) {
		const [err, response] = await asaw(fn());

		if (err) {
			setError(err.toString());
			return;
		}

		if (!response.ok) {
			setError(response.error);
			return;
		}

		posthog?.capture(
			type === "login" ? CLIENT_EVENTS.LOGIN : CLIENT_EVENTS.REGISTER
		);
		window.location.replace(callbackUrl);
	}

	async function handleGoogleSignIn() {
		setIsGoogleLoading(true);
		setError("");

		try {
			// For Google OAuth, we need to allow natural redirects
			const result = await signIn("google", {
				callbackUrl,
				redirect: false,
				// Don't set redirect: false for OAuth providers
			});

			// If we reach here, there might be an error
			if (result?.error) {
				console.error("Google sign-in error:", result.error);
				setError(result.error);
			}
		} catch (err) {
			console.error("Google sign-in exception:", err);
			setError("Failed to sign in with Google");
		} finally {
			setIsGoogleLoading(false);
		}
	}

	async function handleGithubSignIn() {
		setIsGithubLoading(true);
		setError("");

		try {
			const result = await signIn("github", {
				callbackUrl,
				redirect: false,
			});

			// If we reach here, there might be an error
			if (result?.error) {
				console.error("Github sign-in error:", result.error);
				setError(result.error);
			}
		} catch (err) {
			console.error("Github sign-in exception:", err);
			setError("Failed to sign in with Github");
		} finally {
			setIsGithubLoading(false);
		}
	}

	async function login(formData: FormData) {
		authWrapper(() =>
			signIn("login", {
				callbackUrl,
				email: formData.get("email") as string,
				password: formData.get("password") as string,
				redirect: false,
			})
		);
	}

	async function register(formData: FormData) {
		authWrapper(() =>
			signIn("register", {
				callbackUrl,
				email: formData.get("email") as string,
				password: formData.get("password") as string,
				redirect: false,
			})
		);
	}

	return (
		<div className="mx-auto grid gap-6 w-[350px] text-stone-900">
			<div className="grid gap-2 text-center">
			</div>
			{error && <SignInError error={error} />}

			{/* Google Sign-in Button */}
			{providers.includes("google") && (
				<Button
					type="button"
					variant="outline"
					onClick={handleGoogleSignIn}
					disabled={isGoogleLoading}
					className="w-full bg-white dark:bg-white text-stone-900 dark:text-stone-900 border-stone-200 dark:border-stone-200 hover:bg-stone-50 dark:hover:bg-stone-50 rounded-full hover:bg-stone-50 dark:hover:bg-stone-50 dark:hover:text-stone-900 hover:text-stone-900"
				>
					{isGoogleLoading ? (
						"Signing in..."
					) : (
						<div className="flex items-center justify-center gap-2">
							<svg
								className="w-4 h-4"
								viewBox="0 0 24 24"
								xmlns="http://www.w3.org/2000/svg"
							>
								<path
									fill="#4285F4"
									d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
								/>
								<path
									fill="#34A853"
									d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
								/>
								<path
									fill="#FBBC05"
									d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
								/>
								<path
									fill="#EA4335"
									d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
								/>
							</svg>
							Continue with Google
						</div>
					)}
				</Button>
			)}

			{providers.includes("github") && (
				<Button
					type="button"
					variant="outline"
					onClick={handleGithubSignIn}
					disabled={isGithubLoading}
					className="w-full bg-white dark:bg-white text-stone-900 dark:text-stone-900 dark:hover:text-stone-900 border-stone-200 dark:border-stone-200 hover:bg-stone-50 dark:hover:bg-stone-50 rounded-full hover:bg-stone-50 dark:hover:bg-stone-50 hover:dark:bg-stone-50 hover:dark:text-stone-900 hover:text-stone-900"
				>
					{isGithubLoading ? (
						"Signing in..."
					) : (
						<div className="flex items-center justify-center gap-2">
							<svg
								className="w-4 h-4"
								viewBox="0 0 24 24"
								xmlns="http://www.w3.org/2000/svg"
							>
								<path
									fill="#181717"
									d="M12 0C5.374 0 0 5.373 0 12c0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.509 11.509 0 0112 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576C20.566 21.797 24 17.3 24 12c0-6.627-5.373-12-12-12z"
								/>
							</svg>
							Continue with Github
						</div>
					)}
				</Button>
			)}

			{/* Divider */}
			{
				providers.length > 0 && (
					<div className="relative">
						<div className="absolute inset-0 flex items-center">
							<span className="w-full border-t border-stone-300" />
						</div>
						<div className="relative flex justify-center text-xs uppercase">
							<span className="bg-white px-2 text-stone-500">Or</span>
						</div>
					</div>
				)
			}

			<form
				action={type === "login" ? login : register}
				className="flex flex-col"
			>
				<div className="grid gap-4">
					<div className="grid gap-2">
						<Label htmlFor="email">Email</Label>
						<Input
							className="ph-no-capture dark:bg-white dark:border-stone-200"
							id="email"
							name="email"
							type="email"
							placeholder="user@openlit.io"
							autoComplete="email"
							required
						/>
					</div>
					<div className="grid gap-2">
						<div className="flex items-center">
							<Label htmlFor="password">Password</Label>
						</div>
						<Input
							className="ph-no-capture dark:bg-white dark:border-stone-200"
							autoComplete="current-password"
							id="password"
							name="password"
							type="password"
							placeholder="********"
							required
						/>
					</div>
					<Button
						type="submit"
						className="w-full bg-primary dark:bg-primary text-white dark:text-white hover:dark:bg-primary rounded-full dark:hover:bg-stone-900/90"
					>
						{type === "login" ? "Sign in" : "Sign Up"}
					</Button>
				</div>
			</form>
			<div className="text-center text-sm">
				{type === "login"
					? "Don't have an account? "
					: "Already have an account?"}{" "}
				<Link
					href={type === "login" ? "/register" : "/login"}
					className="ml-auto inline-block text-sm underline text-primary"
				>
					{type === "login" ? " Sign up" : " Sign in"}
				</Link>
			</div>
		</div>
	);
}
