import { NextMiddleware } from 'next/server';
import { NextResponse } from 'next/server';
import { MiddlewareFactory } from './middlewareFactory';

export function chain(
  functions: MiddlewareFactory[],
  index = 0
): NextMiddleware {
  const current = functions[index];

  if (current) {
    const next = chain(functions, index + 1);
    return current(next);
  }

  return NextResponse.next;
}