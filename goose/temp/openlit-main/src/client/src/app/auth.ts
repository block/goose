import NextAuth, { AuthOptions } from "next-auth";
import GoogleProvider from "next-auth/providers/google";
import GithubProvider from "next-auth/providers/github";
import CredentialsProvider from "next-auth/providers/credentials";
import { PrismaAdapter } from "@next-auth/prisma-adapter";
import { PrismaClient } from "@prisma/client";
import asaw from "@/utils/asaw";
import {
	createNewUser,
	getUserByEmail,
	updateUser,
	doesPasswordMatches,
	getUserById,
} from "@/lib/user";

const prisma = new PrismaClient();

export const authOptions = {
	adapter: PrismaAdapter(prisma),
	callbacks: {
		async jwt({ token, account, user }) {
			// Persist the OAuth access_token and or the user id to the token right after signin
			if (account) {
				token.accessToken = account.access_token;
			}

			if (user?.id) {
				token.id = user.id;
			}

			// Validate that the user still exists in the database for existing tokens
			// This prevents issues when starting with a fresh database but stale cookies
			if (token?.id && !user) {
				try {
					const [, existingUser] = await asaw(
						getUserById({ id: token.id as string })
					);

					// If user doesn't exist in database, invalidate the token
					if (!existingUser) {
						return null;
					}
				} catch (error) {
					// If there's a database connection error during startup,
					// allow the token to pass through to avoid blocking the app
					// The error will be handled at the application level
					console.error("Database error during JWT validation:", error);
				}
			}

			return token;
		},
		signIn: async ({ user, account, profile }) => {
			if (!user.email) {
				return false;
			}

			// For Google provider, handle account linking
			if (account?.provider === "google" || account?.provider === "github") {
				const [, existingUser] = await asaw(
					getUserByEmail({ email: user.email })
				);

				// If user exists but doesn't have Google account linked
				if (existingUser) {
					try {
						// Check if Provider's account is already linked
						const existingAccount = await prisma.account.findFirst({
							where: {
								userId: existingUser.id,
								provider: account?.provider
							}
						});

						// If Provider's account not linked, link it manually
						if (!existingAccount) {
							await prisma.account.create({
								data: {
									userId: existingUser.id,
									type: account.type,
									provider: account.provider,
									providerAccountId: account.providerAccountId,
									access_token: account.access_token,
									expires_at: account.expires_at,
									id_token: account.id_token,
									refresh_token: account.refresh_token,
									scope: account.scope,
									token_type: account.token_type,
								}
							});
						}

						// Update user info if name/image is missing
						if (!existingUser.name || !existingUser.image) {
							const updateData: { name?: string; image?: string } = {};
							
							// Handle different profile structures for different providers
							if (account.provider === "google") {
								updateData.name = profile?.name || existingUser.name;
								// @ts-ignore - this is a bug in the types, `picture` is a valid on the `Profile` type
								updateData.image = profile?.picture || existingUser.image;
							} else if (account.provider === "github") {
								// @ts-ignore - this is a bug in the types, `login` is a valid on the `Profile` type
								updateData.name = profile?.name || profile?.login || existingUser.name;
								// @ts-ignore - this is a bug in the types, `avatar_url` is a valid on the `Profile` type
								updateData.image = profile?.avatar_url || existingUser.image;
							}

							if (Object.keys(updateData).length > 0) {
								await asaw(
									updateUser({
										where: { email: user.email },
										data: updateData
									})
								);
							}
						}

					// Set the user ID to the existing user
					user.id = existingUser.id;
				} catch (error) {
					// Use separate parameters to prevent log injection
					console.error('Error linking account for provider:', account.provider, error);
					return false;
				}
				}
				return true;
			}

			return true;
		},
		async session({ session, token }: { session: any; token: any }) {
			try {
				session.user.id = token.id;
			} catch (e) {
				console.log(e);
			}
			return session;
		},
	},
	events: {
		async createUser({ user }) {
			// Run custom setup for new users
			if (user.email && user.id) {
				try {
					const { moveSharedDBConfigToDBUser } = await import("@/lib/db-config");
					await moveSharedDBConfigToDBUser(user.email, user.id);
				} catch (error) {
					console.error("Error moving shared db configs to new user:", error);
				}
			}
		}
	},
	providers: [
		...(process.env.GOOGLE_CLIENT_ID && process.env.GOOGLE_CLIENT_SECRET
			? [
				GoogleProvider({
					clientId: process.env.GOOGLE_CLIENT_ID,
					clientSecret: process.env.GOOGLE_CLIENT_SECRET,
				}),
			]
			: []),
		...(process.env.GITHUB_CLIENT_ID && process.env.GITHUB_CLIENT_SECRET
			? [
				GithubProvider({
					clientId: process.env.GITHUB_CLIENT_ID,
					clientSecret: process.env.GITHUB_CLIENT_SECRET,
				}),
			]
			: []),
		CredentialsProvider({
			id: "login",
			name: "Credentials Login",
			credentials: {
				email: {
					label: "Email",
					type: "email",
					placeholder: "openlit@openlit.io",
				},
				password: { label: "Password", type: "password" },
			},
			async authorize(credentials) {
				if (!credentials) return null;
				const [err, user] = await asaw(
					getUserByEmail({ email: credentials.email, selectPassword: true })
				);
				if (!user || err) return err || "No such user exists!";
				const passwordsMatch = await doesPasswordMatches(
					credentials.password,
					user.password
				);

				if (passwordsMatch) return user;
				return null;
			},
		}),
		CredentialsProvider({
			id: "register",
			name: "Credentials Register",
			credentials: {
				email: {
					label: "Email",
					type: "email",
					placeholder: "openlit@openlit.io",
				},
				password: { label: "Password", type: "password" },
			},
			async authorize(credentials) {
				if (!credentials?.email || !credentials?.password) return null;
				const [err, user] = await asaw(
					createNewUser({
						email: credentials?.email,
						password: credentials?.password,
					})
				);

				if (err) throw new Error(err);
				return user;
			},
		}),
	],
	pages: {
		signIn: "/login",
		newUser: "/register",
		error: "/login", // Redirect errors back to login page
	},
	session: { strategy: "jwt" },
} as AuthOptions;

const handler = NextAuth(authOptions);

export { handler as GET, handler as POST };