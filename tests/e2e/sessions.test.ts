import fs from 'node:fs'
import path from 'node:path'
import { expect, test } from '@microsoft/tui-test'
import { createFixtureSession, createTempEnv } from './helpers.js'

const BIN = path.resolve(
  process.cwd(),
  '../../target/debug/agent-session-manager',
)

// ─── Test 1: shows sessions on start ───────────────────────────────────────

const env1 = createTempEnv()
createFixtureSession(env1.claudeDir, '-my-project', 'uuid-001', [
  ['user', 'Hello World'],
])

test.describe('shows sessions on start', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env1.claudeDir,
      AGENT_CONFIG_DIR: env1.configDir,
    },
    rows: 24,
    columns: 80,
  })

  test('shows sessions on start', async ({ terminal }) => {
    await expect(
      terminal.getByText('Sessions', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    await expect(
      terminal.getByText('my-project', { strict: false }),
    ).toBeVisible()
  })
})

// ─── Test 2: search filters list ───────────────────────────────────────────

const env2 = createTempEnv()
createFixtureSession(env2.claudeDir, '-alpha-project', 'uuid-a', [
  ['user', 'rust code'],
])
createFixtureSession(env2.claudeDir, '-beta-project', 'uuid-b', [
  ['user', 'python code'],
])

test.describe('search filters list', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env2.claudeDir,
      AGENT_CONFIG_DIR: env2.configDir,
    },
    rows: 24,
    columns: 80,
  })

  test('search filters list', async ({ terminal }) => {
    await expect(
      terminal.getByText('Sessions', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('\x06') // Ctrl+F
    await expect(
      terminal.getByText('Search', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('alpha')
    await expect(
      terminal.getByText('alpha-project', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
  })
})

// ─── Test 3: settings change export path ────────────────────────────────────

const env3 = createTempEnv()

test.describe('settings change export path', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env3.claudeDir,
      AGENT_CONFIG_DIR: env3.configDir,
    },
    rows: 24,
    columns: 80,
  })

  test('settings change export path', async ({ terminal }) => {
    await expect(
      terminal.getByText('Sessions', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('g')
    await expect(
      terminal.getByText('Export Path', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.keyBackspace(30)
    const newPath = env3.exportDir.replace(/\\/g, '/')
    terminal.write(newPath)
    terminal.submit()
    await expect(
      terminal.getByText('Settings saved', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    const cfgPath = path.join(env3.configDir, 'config.json')
    expect(fs.existsSync(cfgPath)).toBe(true)
  })
})

// ─── Test 4: export creates file ────────────────────────────────────────────

const env4 = createTempEnv()
createFixtureSession(env4.claudeDir, '-export-project', 'uuid-e', [
  ['user', 'export this message'],
])
fs.writeFileSync(
  path.join(env4.configDir, 'config.json'),
  JSON.stringify({ export_path: env4.exportDir }),
)

test.describe('export creates file', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env4.claudeDir,
      AGENT_CONFIG_DIR: env4.configDir,
    },
    rows: 24,
    columns: 80,
  })

  test('export creates file', async ({ terminal }) => {
    await expect(
      terminal.getByText('export-project', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('e')
    await expect(
      terminal.getByText('Exported', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    const files = fs.readdirSync(env4.exportDir)
    expect(files.length).toBe(1)
  })
})

// ─── Test 5: delete moves to trash tab ──────────────────────────────────────

const env5 = createTempEnv()
createFixtureSession(env5.claudeDir, '-delete-me', 'uuid-d', [
  ['user', 'to be deleted'],
])

test.describe('delete moves to trash tab', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env5.claudeDir,
      AGENT_CONFIG_DIR: env5.configDir,
    },
    rows: 24,
    columns: 80,
  })

  test('delete moves to trash tab', async ({ terminal }) => {
    await expect(
      terminal.getByText('delete-me', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('d')
    await expect(
      terminal.getByText('trash', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('y')
    terminal.write('\t')
    await expect(
      terminal.getByText('Trash', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    await expect(
      terminal.getByText('delete-me', { strict: false }),
    ).toBeVisible()
  })
})

// ─── Test 6: restore from trash ─────────────────────────────────────────────

const env6 = createTempEnv()
createFixtureSession(env6.claudeDir, '-restore-me', 'uuid-r', [
  ['user', 'restore me'],
])

test.describe('restore from trash', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env6.claudeDir,
      AGENT_CONFIG_DIR: env6.configDir,
    },
    rows: 24,
    columns: 80,
  })

  test('restore from trash', async ({ terminal }) => {
    await expect(
      terminal.getByText('restore-me', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('d')
    await expect(
      terminal.getByText('trash', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('y')
    terminal.write('\t')
    await expect(
      terminal.getByText('Trash', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    await expect(
      terminal.getByText('restore-me', { strict: false }),
    ).toBeVisible()
    terminal.write('r')
    await expect(
      terminal.getByText('Restored', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
  })
})

// ─── Test 7: sort changes order ──────────────────────────────────────────────

const env7 = createTempEnv()
createFixtureSession(env7.claudeDir, '-zzz-project', 'uuid-z', [
  ['user', 'one'],
])
createFixtureSession(env7.claudeDir, '-aaa-project', 'uuid-a', [
  ['user', 'one'],
  ['user', 'two'],
  ['user', 'three'],
])

test.describe('sort changes order', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env7.claudeDir,
      AGENT_CONFIG_DIR: env7.configDir,
    },
    rows: 24,
    columns: 80,
  })

  test('sort changes order', async ({ terminal }) => {
    await expect(
      terminal.getByText('Sessions', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('s') // toggle sort to Project
    await expect(
      terminal.getByText('Sorted by', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
  })
})

// ─── Test 8: help modal opens and closes ─────────────────────────────────────

const env8 = createTempEnv()

test.describe('help modal opens and closes', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env8.claudeDir,
      AGENT_CONFIG_DIR: env8.configDir,
    },
    rows: 24,
    columns: 80,
  })

  test('help modal opens and closes', async ({ terminal }) => {
    await expect(
      terminal.getByText('Sessions', { strict: false }),
    ).toBeVisible()
    terminal.write('h')
    await expect(
      terminal.getByText('Help', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
    terminal.write('h') // close
    await expect(
      terminal.getByText('Sessions', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()
  })
})
