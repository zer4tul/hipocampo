import { HipocampoAdapter } from './adapter';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

describe('HipocampoAdapter', () => {
  let adapter: HipocampoAdapter;
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'hipocampo-test-'));
    adapter = new HipocampoAdapter({ workspace: tempDir });
  });

  afterEach(() => {
    adapter.close();
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  test('stores and retrieves memory', async () => {
    const id = await adapter.store('test-key', 'Test content', 'core');
    expect(id).toBeDefined();

    const entry = await adapter.get('test-key');
    expect(entry).not.toBeNull();
    expect(entry?.content).toBe('Test content');
    expect(entry?.category).toBe('core');
  });

  test('searches memories', async () => {
    await adapter.store('key1', 'Hello world', 'core');
    await adapter.store('key2', 'Hello rust', 'core');

    const results = await adapter.search('Hello');
    expect(results.length).toBeGreaterThan(0);
    expect(results[0].content).toContain('Hello');
  });

  test('lists memories with filter', async () => {
    await adapter.store('key1', 'Content 1', 'core');
    await adapter.store('key2', 'Content 2', 'daily');

    const coreEntries = await adapter.list({ category: 'core' });
    expect(coreEntries.length).toBe(1);
    expect(coreEntries[0].category).toBe('core');
  });

  test('forgets memory', async () => {
    await adapter.store('test-key', 'Test content', 'core');

    const deleted = await adapter.forget('test-key');
    expect(deleted).toBe(true);

    const entry = await adapter.get('test-key');
    expect(entry).toBeNull();
  });

  test('counts memories', async () => {
    await adapter.store('key1', 'Content 1', 'core');
    await adapter.store('key2', 'Content 2', 'core');

    const count = await adapter.count();
    expect(count).toBe(2);
  });

  test('health check passes', async () => {
    const healthy = await adapter.healthCheck();
    expect(healthy).toBe(true);
  });
});
