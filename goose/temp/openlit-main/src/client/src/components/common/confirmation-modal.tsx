import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";

type ConfirmationModalProps = {
	title: string;
	subtitle: string;
	children: JSX.Element;
	handleYes: (p?: any) => void;
	handleNo?: () => void;
	params?: any;
};

export default function ConfirmationModal(props: ConfirmationModalProps) {
	return (
		<Dialog>
			<DialogTrigger asChild>{props.children}</DialogTrigger>
			<DialogContent className="">
				<DialogHeader>
					<DialogTitle>{props.title}</DialogTitle>
					<DialogDescription>{props.subtitle}</DialogDescription>
				</DialogHeader>
				<DialogFooter className="justify-end">
					<DialogClose asChild>
						<Button variant="secondary" onClick={props.handleNo}>
							No, cancel
						</Button>
					</DialogClose>

					<Button
						variant="default"
						onClick={() => props.handleYes(props.params)}
					>
						Yes, I&apos;m sure
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
