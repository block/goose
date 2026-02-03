import { NextFetchEvent, NextMiddleware, NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';
import { RESTRICTED_DEMO_ACCOUNT_ROUTES } from '@/constants/route';
import { getToken } from 'next-auth/jwt';


export default function checkDemoAccount(next: NextMiddleware) {
  return async (
    request: NextRequest,
    _next: NextFetchEvent
  ) => {
    if (process.env.DEMO_ACCOUNT_EMAIL) {
      const pathname = request.nextUrl.pathname;
      if (RESTRICTED_DEMO_ACCOUNT_ROUTES[request.method]?.some((regex) => (new RegExp(regex)).test(pathname))) {
        const token = await getToken({ req: request });
        if (process.env.DEMO_ACCOUNT_EMAIL.toLowerCase() === token?.email?.toLowerCase()) {
          return NextResponse.json("This Action is not allowed for demo accounts!",
            { status: 403 }
          );
        }
      }
    }

    return next(request, _next);
  };
}