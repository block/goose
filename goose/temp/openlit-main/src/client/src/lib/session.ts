import { getServerSession } from "next-auth/next";
import { authOptions } from "@/app/auth";
import asaw from "@/utils/asaw";
import { getUserById } from "./user";
import { User } from "@prisma/client";

export async function getCurrentUser({
	selectPassword,
}: {
	selectPassword?: boolean;
} = {}): Promise<User | null> {
	const [, session] = await asaw(getServerSession(authOptions));

	if (session?.user?.id) {
		const [, user] = await asaw(
			getUserById({ id: session.user.id, selectPassword })
		);

		return user ?? null;
	}

	return null;
}
