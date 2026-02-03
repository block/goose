export type DatabaseConfig = {
	id: string;
	name: string;
	environment: string;
	username: string;
	password?: string;
	host: string;
	port: string;
	database: string;
	query?: string;
};

export type DatabaseConfigPermissions = {
	canEdit?: boolean;
	canDelete?: boolean;
	canShare?: boolean;
};

export type DatabaseConfigWithActive = DatabaseConfig & {
	isCurrent?: boolean;
	permissions?: DatabaseConfigPermissions;
};
