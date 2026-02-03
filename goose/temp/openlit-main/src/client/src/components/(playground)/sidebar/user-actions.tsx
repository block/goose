import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { getUserDetails, resetUser } from "@/selectors/user";
import { useRootStore } from "@/store";
import { ChevronsUpDown, Cog, LogOut, Pencil } from "lucide-react";
import { signOut } from "next-auth/react";
import Link from "next/link";
import { usePostHog } from "posthog-js/react";
import { useEffect } from "react";

const DROPDOWN_MENU_ITEM_CLASS = "flex w-full text-xs gap-2 hover:bg-primary/10 dark:hover:bg-primary/10 px-2 py-1.5";
const DROPDOWN_MENU_ICON_CLASS = "w-4 h-4";

export default function UserActions() {
	const posthog = usePostHog();
	const user = useRootStore(getUserDetails);
	const resetUserFn = useRootStore(resetUser);
	const onClickSignout = () => {
		posthog?.reset();
		signOut();
		resetUserFn();
	};

	useEffect(() => {
		if (user?.id) {
			posthog?.identify(user.id);
		}
	}, [user]);

	if (!user) return null;

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button variant="outline" className="flex gap-2 justify-start group-data-[state=close]:justify-center p-[calc(0.625rem-1px)] overflow-hidden text-stone-500 dark:text-stone-300 hover:bg-stone-700 dark:hover:bg-stone-600 hover:text-white font-normal w-full">
					<Avatar className={"h-5 w-5 rounded-lg"}>
						<AvatarImage className="rounded-full" src={user!.image || ""} alt={user!.name || ""} />
						<AvatarFallback className="rounded-lg bg-transparent dark:bg-transparent">{user!.name?.substring(0, 2) || user!.email?.substring(0, 2)}</AvatarFallback>
					</Avatar>
					<div className="grid flex-1 text-left text-xs leading-tight group-data-[state=close]:hidden text-ellipsis overflow-hidden whitespace-nowrap">
						<span className="truncate font-medium">{user!.name}</span>
						<span className="truncate text-xs">{user!.email}</span>
					</div>
					<ChevronsUpDown className={`size-4 block group-data-[state=close]:hidden shrink-0 self-center`} />
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent
				className="w-(--radix-dropdown-menu-trigger-width) min-w-56 rounded-lg"
				side="right"
				align="end"
				sideOffset={4}
			>
				<DropdownMenuLabel className="p-0 font-normal">
					<div className="flex items-center gap-2 px-1 text-left text-sm">
						<Avatar className="h-8 w-8 rounded-lg">
							<AvatarImage src={user!.image || ""} alt={user!.name || ""} />
							<AvatarFallback className="rounded-lg">{user!.name?.substring(0, 2) || user!.email?.substring(0, 2)}</AvatarFallback>
						</Avatar>
						<div className="grid flex-1 text-left text-xs leading-tight text-ellipsis overflow-hidden whitespace-nowrap">
							<span className="truncate font-medium">{user!.name}</span>
							<span className="truncate text-xs">{user!.email}</span>
						</div>
					</div>
				</DropdownMenuLabel>
				<DropdownMenuSeparator />
				<DropdownMenuGroup className="gap-1 flex flex-col w-full">
					<DropdownMenuItem className="p-0">
						<Link href="/settings/profile" className={DROPDOWN_MENU_ITEM_CLASS}>
							<Pencil className={DROPDOWN_MENU_ICON_CLASS} />
							Edit details
						</Link>
					</DropdownMenuItem>
				</DropdownMenuGroup>
				<DropdownMenuSeparator />
				<DropdownMenuItem className="p-0" onClick={onClickSignout}>
					<div className={DROPDOWN_MENU_ITEM_CLASS}>
						<LogOut className={DROPDOWN_MENU_ICON_CLASS} />
						Log out
					</div>
				</DropdownMenuItem>
			</DropdownMenuContent>
		</DropdownMenu>
	)
}