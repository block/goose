export interface Secret {
	id: string;
	key: string;
	value: string;
	tags: string[];
	created_by: string;
	created_at: string;
	updated_by: string;
	updated_at: string;
}

export interface SecretInput extends Omit<Secret, "id"> {
	id?: string;
}

export interface SecretGetFilters {
	databaseConfigId?: string;
	tags?: string[];
	key?: string;
}

export interface SecretGetFiltersWithApiKey extends SecretGetFilters {
	apiKey: string;
}
