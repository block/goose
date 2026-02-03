import { compare, genSaltSync, hashSync } from "bcrypt-ts";
import prisma from "./prisma";
import asaw from "@/utils/asaw";
import { getCurrentUser } from "./session";
import { User } from "@prisma/client";
import { moveSharedDBConfigToDBUser } from "./db-config";
import getMessage from "@/constants/messages";

function exclude<User extends Record<string, unknown>, K extends keyof User>(
	user: User,
	keys: K[] = ["password"] as K[]
): Omit<User, K> {
	return Object.fromEntries(
		Object.entries(user).filter(([key]) =>
			typeof keys.includes === "function" ? !keys.includes(key as K) : true
		)
	) as Omit<User, K>;
}

export const getUserByEmail = async ({
	email,
	selectPassword = false,
}: {
	email?: string;
	selectPassword?: boolean;
}) => {
	if (!email) throw new Error("No email Provided");
	const user = await prisma.user.findUnique({
		where: {
			email,
		},
	});

	if (!user) throw new Error("No user with this email exists");

	return exclude(user, selectPassword ? [] : undefined);
};

export const getUserById = async ({
	id,
	selectPassword = false,
}: {
	id?: string;
	selectPassword?: boolean;
}) => {
	if (!id) return null;
	const user = await prisma.user.findUnique({
		where: {
			id,
		},
	});

	if (!user) return null;

	return exclude(user, selectPassword ? [] : undefined);
};

export const createNewUser = async (
	{
		email,
		password,
	}: {
		email: string;
		password: string;
	},
	options?: { selectPassword?: boolean }
) => {
	const [, existingUser] = await asaw(getUserByEmail({ email }));
	if (existingUser) throw new Error("User already exists! Please signin!");

	const hashedPassword = getHashedPassword(password);

	let createdUser = await prisma.user.create({
		data: {
			email,
			password: hashedPassword,
		},
	});

	if (createdUser?.id) {
		await moveSharedDBConfigToDBUser(email, createdUser.id);
		return exclude(createdUser, options?.selectPassword ? [] : undefined);
	}

	throw new Error("Cannot create a user!");
};

export const updateUser = async ({
	data,
	where,
}: {
	data: any;
	where: any;
}) => {
	if (!where || !Object.keys(where).length)
		throw new Error("No where clause defined");
	return await prisma.user.update({
		where,
		data,
	});
};

export const updateUserProfile = async ({
	currentPassword,
	newPassword,
	name,
}: {
	currentPassword?: string;
	newPassword?: string;
	name?: string;
}) => {
	const user = await getCurrentUser({ selectPassword: true });

	if (!user) throw new Error(getMessage().UNAUTHORIZED_USER);

	const updatedUserObject: Partial<User> = {};

	if (newPassword) {
		if (!currentPassword)
			throw new Error("Provide current password to update it to new one!");
		const passwordsMatch = await doesPasswordMatches(
			currentPassword,
			user.password || ""
		);
		if (!passwordsMatch) throw new Error("Wrong current password!");
		updatedUserObject.password = getHashedPassword(newPassword);
	}

	if (name) {
		updatedUserObject.name = name;
	}

	if (Object.keys(updatedUserObject).length === 0)
		throw new Error("Nothing to update!");

	return updateUser({
		data: updatedUserObject,
		where: { id: user.id },
	});
};

const getHashedPassword = (password: string): string => {
	const salt = genSaltSync(10);
	const hash = hashSync(password, salt);
	return hash;
};

export const doesPasswordMatches = async (
	password: string,
	userPassword: string
): Promise<boolean> => {
	return await compare(password, userPassword);
};
