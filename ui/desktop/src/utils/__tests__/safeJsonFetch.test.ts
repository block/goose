import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { safeJsonFetch } from '../safeJsonFetch';

describe('safeJsonFetch', () => {
  const fetchMock = vi.fn<typeof fetch>();

  beforeEach(() => {
    vi.stubGlobal('fetch', fetchMock);
    fetchMock.mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns original response for non-empty JSON body', async () => {
    const response = new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    });
    fetchMock.mockResolvedValue(response);

    const result = await safeJsonFetch('/test');

    expect(result).toBe(response);
    expect(await result.json()).toEqual({ ok: true });
  });

  it('synthesizes {} for empty JSON body without content-length=0', async () => {
    const response = new Response('', {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    });
    fetchMock.mockResolvedValue(response);

    const result = await safeJsonFetch('/test');

    expect(result).not.toBe(response);
    expect(await result.text()).toBe('{}');
    expect(result.headers.get('Content-Length')).toBe('2');
    expect(result.status).toBe(200);
  });

  it('passes through 204 responses unchanged', async () => {
    const response = new Response(null, {
      status: 204,
      headers: { 'Content-Type': 'application/json' },
    });
    fetchMock.mockResolvedValue(response);

    const result = await safeJsonFetch('/test');
    expect(result).toBe(response);
  });

  it('passes through 205 responses unchanged', async () => {
    const response = new Response(null, {
      status: 205,
      headers: { 'Content-Type': 'application/json' },
    });
    fetchMock.mockResolvedValue(response);

    const result = await safeJsonFetch('/test');
    expect(result).toBe(response);
  });

  it('passes through content-length=0 responses unchanged', async () => {
    const response = new Response('', {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': '0',
      },
    });
    fetchMock.mockResolvedValue(response);

    const result = await safeJsonFetch('/test');
    expect(result).toBe(response);
  });

  it('passes through non-JSON responses unchanged', async () => {
    const response = new Response('', {
      status: 200,
      headers: { 'Content-Type': 'text/plain' },
    });
    fetchMock.mockResolvedValue(response);

    const result = await safeJsonFetch('/test');
    expect(result).toBe(response);
  });
});
