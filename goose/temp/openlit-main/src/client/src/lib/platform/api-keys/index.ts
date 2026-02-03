import crypto from "crypto";
import { getDBConfigByUser } from "@/lib/db-config";
import asaw from "@/utils/asaw";
import prisma from "@/lib/prisma";
import { getCurrentUser } from "@/lib/session";
import { throwIfError } from "@/utils/error";
import getMessage from "@/constants/messages";

const APIKEY_PREFIX = "openlit-";

function createAPIKey() {
	// Generate 32 random bytes
	const key = crypto.randomBytes(32);

	// Convert the byte array to a Base64 string
	return `${APIKEY_PREFIX}${key.toString("base64")}`;
}

export async function generateAPIKey(name: string) {
	const user = await getCurrentUser();

	throwIfError(!user, getMessage().UNAUTHORIZED_USER);

	const [err, dbConfig] = await asaw(getDBConfigByUser(true));

	throwIfError(err, err);
	throwIfError(!dbConfig?.id, getMessage().DATABASE_CONFIG_NOT_FOUND);

	const apiKey = createAPIKey();

	await prisma.aPIKeys.create({
		data: {
			apiKey,
			name,
			databaseConfigId: dbConfig.id,
			createdByUserId: user!.id,
		},
	});

	return {
		apiKey,
		databaseConfigId: dbConfig.id,
	};
}

export async function getAPIKeyInfo({ apiKey }: { apiKey: string }) {
	return await asaw(
		prisma.aPIKeys.findFirst({
			where: {
				AND: [
					{
						apiKey,
					},
					{
						isDeleted: false,
					},
				],
			},
		})
	);
}

export async function getAllAPIKeys() {
	const [err, dbConfig] = await asaw(getDBConfigByUser(true));
	throwIfError(err, err);

	throwIfError(!dbConfig?.id, getMessage().DATABASE_CONFIG_NOT_FOUND);

	const [, data] = await asaw(
		prisma.aPIKeys.findMany({
			where: {
				AND: [
					{
						databaseConfigId: dbConfig?.id,
					},
					{ isDeleted: false },
				],
			},
			select: {
				name: true,
				apiKey: true,
				createdAt: true,
				id: true,
				createdByUser: {
					select: {
						email: true,
					},
				},
			},
			orderBy: {
				createdAt: "desc",
			},
		})
	);
	return data;
}

export async function deleteAPIKey(id: string) {
	return await asaw(
		prisma.aPIKeys.update({ where: { id }, data: { isDeleted: true } })
	);
}
