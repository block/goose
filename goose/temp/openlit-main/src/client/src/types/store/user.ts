import { User } from "@prisma/client";

export type UserStore = {
	details?: User;
	isFetched: boolean;
	set: (u: User) => void;
	reset: () => void;
};