import { withAuth } from "next-auth/middleware";
import { getToken } from "next-auth/jwt";
import {
	NextFetchEvent,
	NextMiddleware,
	NextRequest,
	NextResponse,
} from "next/server";
import {
	DEFAULT_LOGGED_IN_ROUTE,
	ALLOWED_OPENLIT_ROUTES_WITHOUT_TOKEN,
	CRON_JOB_ROUTES,
} from "@/constants/route";

export default function checkAuth(next: NextMiddleware) {
	return withAuth(
		async function middleware(request: NextRequest, _next: NextFetchEvent) {
			const pathname = request.nextUrl.pathname;
			if (
				pathname.startsWith("/_next") ||
				pathname.startsWith("/static") ||
				pathname.startsWith("/images")
			) {
				return next(request, _next);
			}

			try {
				const token = await getToken({ req: request });
				const isAuth = !!token;
				const isAllowedRequestWithoutToken =
					ALLOWED_OPENLIT_ROUTES_WITHOUT_TOKEN.includes(pathname);
				const isCronJobRoute = CRON_JOB_ROUTES.includes(pathname);
				const isAuthPage =
					pathname.startsWith("/login") || pathname.startsWith("/register");
				const isApiPage = pathname.startsWith("/api");

				if (isAuthPage) {
					if (isAuth) {
						return NextResponse.redirect(
							new URL(DEFAULT_LOGGED_IN_ROUTE, request.url)
						);
					}

					return NextResponse.next();
				}

				if (isApiPage) {
					if (isAuth || isAllowedRequestWithoutToken || isCronJobRoute) {
						if (isCronJobRoute) {
							const cronJobToken = request.headers.get("X-CRON-JOB");
							if (cronJobToken) {
								return NextResponse.next();
							}
						}
						return NextResponse.next();
					}
				}

				if (!isAuth) {
					let from = pathname;
					if (request.nextUrl.search) {
						from += request.nextUrl.search;
					}

					return NextResponse.redirect(
						new URL(`/login?callbackUrl=${encodeURIComponent(from)}`, request.url)
					);
				}

				return NextResponse.next();
			} catch (error) {
				// If there's an error getting the token (e.g., invalid/corrupted token),
				// treat as unauthenticated and redirect to login
				console.error("Auth middleware error:", error);
				
				const isAuthPage =
					pathname.startsWith("/login") || pathname.startsWith("/register");
				
				if (!isAuthPage) {
					let from = pathname;
					if (request.nextUrl.search) {
						from += request.nextUrl.search;
					}

					return NextResponse.redirect(
						new URL(`/login?callbackUrl=${encodeURIComponent(from)}`, request.url)
					);
				}
				
				return NextResponse.next();
			}
		},
		{
			callbacks: {
				async authorized() {
					// This is a work-around for handling redirect on auth pages.
					// We return true here so that the middleware function above
					// is always called.
					return true;
				},
			},
		}
	);
}
