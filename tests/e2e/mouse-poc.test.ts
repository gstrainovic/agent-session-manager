import path from 'node:path'
import { expect, test } from '@microsoft/tui-test'
import { createFixtureSession, createTempEnv } from './helpers.js'

const BIN = path.resolve(
  process.cwd(),
  '../../target/debug/agent-session-manager',
)

// ─── Mouse POC: SGR escape sequences via terminal.write() ─────────────────
//
// ERGEBNIS: terminal.write() leitet SGR-Maus-Escape-Sequenzen korrekt als
// stdin an den Kindprozess weiter. crossterm erkennt sie und verarbeitet sie
// als MouseEvent. Damit können Maus-Interaktionen in E2E-Tests simuliert werden.
//
// SGR-Format (crossterm default, 1-basierte Koordinaten):
//   Mouse Down:  \x1b[<button;col;rowM
//   Mouse Up:    \x1b[<button;col;rowm  (kleines m)
//   Scroll Up:   \x1b[<64;col;rowM
//   Scroll Down: \x1b[<65;col;rowM
//   button: 0=left, 1=middle, 2=right

const env1 = createTempEnv()
createFixtureSession(env1.claudeDir, '-mouse-test-project', 'uuid-mouse-001', [
  ['user', 'Hello Mouse Test'],
  ['assistant', 'This is a mouse test response'],
])

test.describe('mouse click via SGR escape sequence', () => {
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

  test('SGR mouse click switches to Trash tab', async ({ terminal }) => {
    // Warten bis App bereit ist
    await expect(
      terminal.getByText('Sessions (1)', { strict: false }),
    ).toBeVisible()
    await expect(terminal).toMatchSnapshot()

    // SGR-Mausklick auf den "Trash"-Tab (col=25, row=1, 1-basiert)
    terminal.write('\x1b[<0;25;1M') // Mouse Down (linke Maustaste)
    terminal.write('\x1b[<0;25;1m') // Mouse Up   (linke Maustaste)

    // Prüfen: Trash-Tab ist aktiv (Kommandoleiste zeigt "Enter restore")
    await expect(
      terminal.getByText('Enter restore', { strict: false }),
    ).toBeVisible({ timeout: 2000 })
    await expect(terminal).toMatchSnapshot()
  })
})

// ─── PoC 2: Session-Auswahl via Mausklick ────────────────────────────────────

const env2 = createTempEnv()
createFixtureSession(env2.claudeDir, '-alpha', 'uuid-poc2a', [['user', 'first session']])
createFixtureSession(env2.claudeDir, '-beta', 'uuid-poc2b', [['user', 'second session']])

test.describe('SGR PoC: Session-Auswahl via Mausklick', () => {
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

  test('Klick auf zweite Session-Zeile ändert Preview-Inhalt', async ({ terminal }) => {
    await expect(terminal.getByText('Sessions', { strict: false })).toBeVisible()
    await expect(terminal).toMatchSnapshot()

    // Session-Liste: row=4=Border, row=5=Header, row=6=erste Session, row=7=zweite Session (1-basiert)
    // Klick auf Zeile 7 (zweite Session), col=10 (links in der 30%-Liste)
    terminal.write('\x1b[<0;10;7M')
    terminal.write('\x1b[<0;10;7m')

    await expect(terminal).toMatchSnapshot()
  })
})

// ─── PoC 3: Scroll via SGR ───────────────────────────────────────────────────

const env3 = createTempEnv()
for (let i = 0; i < 5; i++) {
  createFixtureSession(env3.claudeDir, `-proj-${i}`, `uuid-poc3-${i}`, [
    ['user', `session message ${i}`],
  ])
}

test.describe('SGR PoC: Scroll via Mausrad', () => {
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

  test('3x ScrollDown in der Liste bewegt Auswahl nach unten', async ({ terminal }) => {
    await expect(terminal.getByText('Sessions', { strict: false })).toBeVisible()
    await expect(terminal).toMatchSnapshot()

    // 3x ScrollDown links (col=10, row=10, 1-basiert), Button 65 = ScrollDown
    terminal.write('\x1b[<65;10;10M')
    terminal.write('\x1b[<65;10;10M')
    terminal.write('\x1b[<65;10;10M')

    await expect(terminal).toMatchSnapshot()
  })
})

// ─── PoC 4: Settings öffnen via Mausklick auf Command-Bar ───────────────────

const env4 = createTempEnv()

test.describe('SGR PoC: Settings via Command-Bar Mausklick', () => {
  test.use({
    program: { file: BIN },
    env: {
      ...process.env,
      CLAUDE_DATA_DIR: env4.claudeDir,
      AGENT_CONFIG_DIR: env4.configDir,
    },
    rows: 24,
    columns: 120,
  })

  test('Klick auf "g settings" in Command-Bar öffnet Settings-Modal', async ({ terminal }) => {
    await expect(terminal.getByText('Sessions', { strict: false })).toBeVisible()

    // Command-Bar letzte Zeile (row=24, 1-basiert bei 24 rows), 120 Spalten.
    // "↑↓ nav  ←→ focus  │  Enter resume  d delete  e export  0 clean  f search  s sort  g settings  h help  q quit"
    // Position von "g": col=84 (1-basiert)
    terminal.write('\x1b[<0;84;24M')
    terminal.write('\x1b[<0;84;24m')

    await expect(
      terminal.getByText('Export Path', { strict: false }),
    ).toBeVisible({ timeout: 2000 })
    await expect(terminal).toMatchSnapshot()
  })
})
