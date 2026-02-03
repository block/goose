import { getCurrentUser } from "@/lib/session";
import { updateUserProfile } from "@/lib/user";
import asaw from "@/utils/asaw";

export async function GET() {
	const user: any = await getCurrentUser();

	if (!user)
		return Response.json("No user loggedin!", {
			status: 401,
		});

	return Response.json(user);
}

export async function POST(request: Request) {
	const formData = await request.json();
	const formObject = {
		currentPassword: formData.currentPassword,
		newPassword: formData.newPassword,
		name: formData.name,
	};

	const [err, res]: any = await asaw(updateUserProfile(formObject));

	if (err)
		return Response.json(err, {
			status: 400,
		});

	return Response.json(res);
}
