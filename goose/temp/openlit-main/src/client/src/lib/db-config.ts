import asaw from "@/utils/asaw";
import prisma from "./prisma";
import { getCurrentUser } from "./session";
import { DatabaseConfig, DatabaseConfigInvitedUser } from "@prisma/client";
import migrations from "@/clickhouse/migrations";
import getMessage from "@/constants/messages";
import { throwIfError } from "@/utils/error";
import { consoleLog } from "@/utils/log";

export const getDBConfigByUser = async (currentOnly?: boolean) => {
	const user = await getCurrentUser();

	if (!user) throw new Error(getMessage().UNAUTHORIZED_USER);

	if (currentOnly) {
		const dbConfig = await prisma.databaseConfigUser.findFirst({
			where: {
				userId: user.id,
				isCurrent: true,
			},
			select: {
				databaseConfig: true,
			},
		});

		return dbConfig?.databaseConfig;
	}

	const dbUserConfigs = await prisma.databaseConfigUser.findMany({
		where: {
			userId: user.id,
		},
		select: {
			databaseConfigId: true,
			isCurrent: true,
			databaseConfig: true,
			canEdit: true,
			canDelete: true,
			canShare: true,
		},
		orderBy: {
			databaseConfig: {
				createdAt: "asc",
			},
		},
	});

	const dbConfigs = dbUserConfigs.map((dbConfig) => ({
		...dbConfig.databaseConfig,
		permissions: {
			canEdit: dbConfig.canEdit,
			canDelete: dbConfig.canDelete,
			canShare: dbConfig.canShare,
		},
	}));
	const currentConfig = dbUserConfigs.find((dbConfig) => dbConfig.isCurrent);
	return dbConfigs.map((dbConfig) => ({
		...dbConfig,
		isCurrent: currentConfig?.databaseConfigId === dbConfig?.id,
	}));
};

export const getDBConfigById = async ({ id }: { id: string }) => {
	return await prisma.databaseConfig.findUnique({
		where: {
			id,
		},
	});
};

export const upsertDBConfig = async (
	dbConfig: Partial<DatabaseConfig>,
	id?: string
) => {
	if (!dbConfig.name) throw new Error("No name provided");
	if (!dbConfig.username) throw new Error("No username provided");
	if (!dbConfig.host) throw new Error("No host provided");
	if (!dbConfig.port) throw new Error("No port provided");
	if (!dbConfig.database) throw new Error("No database provided");

	const user = await getCurrentUser();

	throwIfError(!user, getMessage().UNAUTHORIZED_USER);

	const existingDBName = await prisma.databaseConfig.findUnique({
		where: {
			name: dbConfig.name,
			NOT: {
				id,
			},
		},
	});

	if (existingDBName?.id) throw new Error("DB config Name already exists");

	const whereObject: any = {};
	if (id) whereObject.id = id;
	else whereObject.name = dbConfig.name;

	if (id) {
		await checkPermissionForDbAction(user!.id, id, "EDIT");
	}

	const [err, createddbConfig] = await asaw(
		prisma.databaseConfig.upsert({
			where: whereObject,
			create: {
				...(dbConfig as any),
				createdByUserId: user!.id,
			},
			update: {
				...dbConfig,
			},
		})
	);

	if (!id) {
		await addDatabaseConfigUserEntry(user!.id, createddbConfig.id, {
			canEdit: true,
			canDelete: true,
			canShare: true,
		});
		migrations(createddbConfig.id);
	}

	return `${id ? "Updated" : "Added"} db details successfully`;
};

export async function deleteDBConfig(id: string) {
	const user = await getCurrentUser();

	if (!user) throw new Error(getMessage().UNAUTHORIZED_USER);

	await checkPermissionForDbAction(user.id, id, "DELETE");

	await prisma.databaseConfigUser.delete({
		where: {
			databaseConfigId_userId: {
				userId: user.id,
				databaseConfigId: id,
			},
		},
	});

	await prisma.databaseConfig.delete({
		where: {
			id,
		},
	});

	return "Deleted successfully!";
}

export async function setCurrentDBConfig(id: string) {
	const user = await getCurrentUser();

	if (!user) throw new Error(getMessage().UNAUTHORIZED_USER);

	const currentConfig = await getDBConfigByUser(true);

	if ((currentConfig as DatabaseConfig)?.id) {
		await prisma.databaseConfigUser.update({
			where: {
				databaseConfigId_userId: {
					userId: user.id,
					databaseConfigId: (currentConfig as DatabaseConfig).id,
				},
			},
			data: {
				isCurrent: false,
			},
		});
	}

	await prisma.databaseConfigUser.update({
		where: {
			databaseConfigId_userId: {
				userId: user.id,
				databaseConfigId: id,
			},
		},
		data: {
			isCurrent: true,
		},
	});

	return "Current DB config set successfully!";
}

export async function shareDBConfig({
	shareArray,
	id,
}: {
	id: string;
	shareArray: {
		email: string;
		permissions?: {
			canDelete: boolean;
			canEdit: boolean;
			canShare: boolean;
		};
	}[];
}) {
	if (!id || !shareArray?.length) throw new Error("No user to share!");

	const user = await getCurrentUser();

	if (!user) throw new Error(getMessage().UNAUTHORIZED_USER);

	const { dbUserConfig } = await checkPermissionForDbAction(
		user.id,
		id,
		"SHARE"
	);

	return await Promise.all(
		shareArray.map(
			async ({
				email,
				permissions = {
					canDelete: false,
					canEdit: false,
					canShare: false,
				},
			}) => {
				const [, user] = await asaw(
					prisma.user.findUnique({
						where: {
							email,
						},
					})
				);

				if (user?.id) {
					const [, dbConfigUser] = await asaw(
						prisma.databaseConfigUser.findFirst({
							where: {
								userId: user.id,
								databaseConfigId: id,
							},
						})
					);

					if (!dbConfigUser) {
						await addDatabaseConfigUserEntry(user.id, id, permissions);
						return [, { success: true }];
					}

					return [`Already shared to ${email}`, { success: false }];
				} else {
					const [createErr] = await asaw(
						prisma.databaseConfigInvitedUser.create({
							data: {
								email,
								databaseConfigId: id,
								canEdit: dbUserConfig.canEdit && permissions.canEdit,
								canDelete: dbUserConfig.canDelete && permissions.canDelete,
								canShare: dbUserConfig.canShare && permissions.canShare,
							},
						})
					);

					return [createErr, { success: !createErr }];
				}
			}
		)
	);
}

export async function moveSharedDBConfigToDBUser(
	email: string,
	userId: string
) {
	const [sharedConfigErr, sharedConfig] = await asaw(
		prisma.databaseConfigInvitedUser.findMany({
			where: {
				email,
			},
		})
	);

	if (sharedConfigErr) {
		consoleLog(sharedConfigErr);
		return;
	}

	if (!sharedConfig?.length) return;

	if (sharedConfig.length) {
		const [configAddErr] = await asaw(
			prisma.databaseConfigUser.createMany({
				data: sharedConfig.map(
					(sharedConfigObject: DatabaseConfigInvitedUser) => ({
						databaseConfigId: sharedConfigObject.databaseConfigId,
						userId,
						canDelete: sharedConfigObject.canDelete,
						canEdit: sharedConfigObject.canEdit,
						canShare: sharedConfigObject.canShare,
					})
				),
			})
		);

		const ifNoCurrentDbConfig = await prisma.databaseConfigUser.count({
			where: {
				userId,
				isCurrent: true,
			},
		});

		if (!ifNoCurrentDbConfig) {
			const firstDbConfig = await prisma.databaseConfigUser.findFirst({
				where: {
					userId,
				},
			});
			if (firstDbConfig) {
				await prisma.databaseConfigUser.update({
					data: {
						isCurrent: true,
					},
					where: {
						databaseConfigId_userId: {
							userId,
							databaseConfigId: firstDbConfig.databaseConfigId,
						},
					},
				});
			}
		}
	}

	return;
}

async function addDatabaseConfigUserEntry(
	userId: string,
	databaseConfigId: string,
	permissions: {
		canDelete: boolean;
		canEdit: boolean;
		canShare: boolean;
	}
) {
	const ifFirstDBConfigCreated = await prisma.databaseConfigUser.count({
		where: {
			userId: userId,
		},
	});
	await prisma.databaseConfigUser.create({
		data: {
			userId,
			databaseConfigId,
			isCurrent: !ifFirstDBConfigCreated,
			...permissions,
		},
	});
}

async function checkPermissionForDbAction(
	userId: string,
	databaseConfigId: string,
	actionType: "DELETE" | "SHARE" | "EDIT"
) {
	const [dbUserConfigErr, dbUserConfig] = await asaw(
		prisma.databaseConfigUser.findFirst({
			where: {
				databaseConfigId,
				userId,
			},
		})
	);

	if (dbUserConfigErr || !dbUserConfig)
		throw new Error(dbUserConfigErr || "Database config doesn't exist");

	switch (actionType) {
		case "DELETE":
			if (!dbUserConfig.canDelete)
				throw new Error(
					"User doesn't have permission to delete the database config"
				);
			break;
		case "EDIT":
			if (!dbUserConfig.canEdit)
				throw new Error(
					"User doesn't have permission to edit the database config"
				);
			break;
		case "EDIT":
			if (!dbUserConfig.canShare)
				throw new Error(
					"User doesn't have permission to share the database config"
				);
			break;
		default:
			break;
	}

	return {
		success: true,
		dbUserConfig,
	};
}
