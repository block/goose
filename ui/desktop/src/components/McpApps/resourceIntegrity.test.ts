import { describe, expect, it } from 'vitest';
import {
  ResourceIntegrityTracker,
  checkResourceIntegrity,
  computeResourceHash,
  resourceIntegrityKey,
} from './resourceIntegrity';

describe('computeResourceHash', () => {
  it('produces the known SHA-256 for a fixed input', async () => {
    expect(await computeResourceHash('abc')).toBe(
      'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
    );
  });

  it('produces different hashes for different content', async () => {
    const a = await computeResourceHash('<html>a</html>');
    const b = await computeResourceHash('<html>b</html>');
    expect(a).not.toBe(b);
  });
});

describe('resourceIntegrityKey', () => {
  it('does not collide across different extension/uri pairs', () => {
    expect(resourceIntegrityKey('ext', 'ui://a')).not.toBe(resourceIntegrityKey('ext', 'ui://ab'));
    expect(resourceIntegrityKey('ext', 'ui://a')).not.toBe(resourceIntegrityKey('exta', 'ui://'));
  });
});

describe('ResourceIntegrityTracker', () => {
  it('marks the first observation as firstSeen', () => {
    const tracker = new ResourceIntegrityTracker();
    const result = tracker.record('k', 'hash1');
    expect(result).toEqual({ hash: 'hash1', firstSeen: true, changed: false });
  });

  it('reports no change when the same hash is recorded again', () => {
    const tracker = new ResourceIntegrityTracker();
    tracker.record('k', 'hash1');
    const result = tracker.record('k', 'hash1');
    expect(result).toEqual({
      hash: 'hash1',
      firstSeen: false,
      changed: false,
      previousHash: 'hash1',
    });
  });

  it('flags a changed hash and reports the first-seen baseline', () => {
    const tracker = new ResourceIntegrityTracker();
    tracker.record('k', 'hash1');
    const result = tracker.record('k', 'hash2');
    expect(result).toEqual({
      hash: 'hash2',
      firstSeen: false,
      changed: true,
      previousHash: 'hash1',
    });
    const repeat = tracker.record('k', 'hash2');
    expect(repeat).toEqual({
      hash: 'hash2',
      firstSeen: false,
      changed: true,
      previousHash: 'hash1',
    });
  });

  it('tracks distinct keys independently', () => {
    const tracker = new ResourceIntegrityTracker();
    expect(tracker.record('a', 'x').firstSeen).toBe(true);
    expect(tracker.record('b', 'y').firstSeen).toBe(true);
    expect(tracker.record('a', 'x').changed).toBe(false);
  });

  it('evicts the oldest baseline when full', () => {
    const tracker = new ResourceIntegrityTracker(2);
    tracker.record('a', 'x');
    tracker.record('b', 'y');
    tracker.record('c', 'z');

    expect(tracker.record('a', 'changed').firstSeen).toBe(true);
    expect(tracker.record('c', 'changed').changed).toBe(true);
  });
});

describe('checkResourceIntegrity', () => {
  it('detects tampering when the served HTML changes across fetches', async () => {
    const tracker = new ResourceIntegrityTracker();

    const first = await checkResourceIntegrity(tracker, 'ext', 'ui://app', '<html>v1</html>');
    expect(first.firstSeen).toBe(true);
    expect(first.changed).toBe(false);

    const same = await checkResourceIntegrity(tracker, 'ext', 'ui://app', '<html>v1</html>');
    expect(same.firstSeen).toBe(false);
    expect(same.changed).toBe(false);

    const tampered = await checkResourceIntegrity(tracker, 'ext', 'ui://app', '<html>evil</html>');
    expect(tampered.changed).toBe(true);
    expect(tampered.previousHash).toBe(first.hash);

    const repeatTampered = await checkResourceIntegrity(
      tracker,
      'ext',
      'ui://app',
      '<html>evil</html>'
    );
    expect(repeatTampered.changed).toBe(true);
    expect(repeatTampered.previousHash).toBe(first.hash);
  });

  it('detects a change to an empty resource', async () => {
    const tracker = new ResourceIntegrityTracker();
    await checkResourceIntegrity(tracker, 'ext', 'ui://app', '<html>content</html>');

    const empty = await checkResourceIntegrity(tracker, 'ext', 'ui://app', '');

    expect(empty.changed).toBe(true);
  });

  it('detects tampering when cached HTML differs from a later fetch', async () => {
    const tracker = new ResourceIntegrityTracker();

    const cached = await checkResourceIntegrity(tracker, 'ext', 'ui://app', '<html>cached</html>');
    expect(cached.firstSeen).toBe(true);

    const fresh = await checkResourceIntegrity(tracker, 'ext', 'ui://app', '<html>fresh</html>');
    expect(fresh.changed).toBe(true);
    expect(fresh.previousHash).toBe(cached.hash);
  });
});
