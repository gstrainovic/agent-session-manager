# Design: UI/UX, Shortcuts & Trash Refactoring

**Datum:** 2026-02-25
**Tasks:** todo.md 1–4

---

## Übersicht

Vier zusammenhängende Verbesserungen:

1. **Trash als Verzeichnis** — `fs::rename()` statt JSON-Serialisierung
2. **Command Bar** — alle Shortcuts sichtbar, kategorisiert
3. **Tab Bar** — aktiver Tab klar markiert
4. **Preview** — Scroll-Indikator

---

## 1. Trash-Architektur

### Problem

`move_selected_to_trash()` löscht die JSONL-Datei und speichert den vollen Inhalt
(`original_content`) in `~/.claude/trash.json`. Bei N gelöschten Sessions mit je
M KB Inhalt wird `save_trash()` bei jedem Delete langsamer (liest + schreibt
die gesamte Datei).

### Lösung

```
Vorher:
  ~/.claude/projects/-home-g-proj/abc123.jsonl  →  GELÖSCHT
  ~/.claude/trash.json                          →  [{ id, content, messages, ... }]

Nachher:
  ~/.claude/projects/-home-g-proj/abc123.jsonl  →  VERSCHOBEN
  ~/.claude/trash/-home-g-proj/abc123.jsonl     →  JSONL-Datei unverändert
```

**Operationen:**
- Delete: `fs::rename(projects/proj/id.jsonl, trash/proj/id.jsonl)`
- Restore: `fs::rename(trash/proj/id.jsonl, projects/proj/id.jsonl)`
- Permanent delete: `fs::remove_file(trash/proj/id.jsonl)`
- Empty trash: `fs::remove_dir_all(trash/)` + neu anlegen

**Session laden:**
- `load_sessions()` liest nur `projects/` — Trash-Verzeichnis wird ignoriert
- `load_trash()` liest `trash/` mit derselben Logik wie `load_sessions()`
- `original_content: Option<String>` Feld auf `Session` entfällt

**Persistenz:** Dateien bleiben bei Neustart erhalten — kein Unterschied zum
bisherigen Verhalten.

---

## 2. Command Bar

### Problem

Eine überfüllte Zeile; Shortcuts `0`, `PgUp/PgDn`, `y/n` fehlen; chaotische
Gruppierung.

### Lösung: 2 Zeilen mit Kategorie-Trennern

```
Sessions-Tab:
┌─────────────────────────────────────────────────────────────────┐
│ ↑↓ nav  ←→ focus │ Enter resume  d delete  e export  0 clean  │
│                   │ s sort  S dir  Ctrl+F search  g settings  q│
└─────────────────────────────────────────────────────────────────┘

Trash-Tab:
┌─────────────────────────────────────────────────────────────────┐
│ ↑↓ nav  ←→ focus │ r restore  d delete  t empty trash          │
│                   │ s sort  S dir  g settings  q quit           │
└─────────────────────────────────────────────────────────────────┘

Confirm-Modus (d gedrückt):
┌─────────────────────────────────────────────────────────────────┐
│  Move 'my-project' to trash?    y/d confirm   n/Esc cancel      │
└─────────────────────────────────────────────────────────────────┘
```

**Layout:** `Constraint::Length(4)` statt `Length(3)` für Command Bar.

---

## 3. Tab Bar

### Problem

Aktiver Tab nur durch Farbe erkennbar — kein struktureller Marker.

### Lösung

```
Vorher:   Sessions (42)    Trash (3)    [Tab] to switch
Nachher:  ● Sessions (42)  ○ Trash (3)  │  Tab: switch  h: help
```

`●` = aktiv (Cyan Bold), `○` = inaktiv (DarkGray).

---

## 4. Preview Scroll-Indikator

### Problem

Kein visuelles Feedback ob/wie viel Inhalt noch scrollbar ist.

### Lösung

Letzte Zeile des Preview-Panels zeigt bei scrollbarem Inhalt:

```
                                              ↓ PgDn (8/23)
```

Format: `↓ PgDn (aktuelle_seite/gesamt_seiten)` — nur sichtbar wenn
`preview_scroll > 0` oder mehr Inhalt vorhanden.

**App-State:** `total_preview_lines: usize` wird beim Rendern berechnet und
im `draw_preview()` verwendet.

---

## Architektur-Änderungen

### `store.rs`

| Methode | Änderung |
|---------|----------|
| `load_sessions()` | Liest nur `projects/`, ignoriert `trash/` |
| `load_trash()` | Liest `trash/` mit gleicher JSONL-Logik |
| `save_trash()` | **entfällt** |
| `delete_session_file()` | Wird zu `move_to_trash()`: `fs::rename()` |
| `restore_session_file()` | Nutzt `fs::rename()` zurück |
| `trash_path` | Wird zu `PathBuf` (Verzeichnis statt Datei) |

### `models.rs`

| Feld | Änderung |
|------|----------|
| `Session.original_content` | **entfällt** |

### `app.rs`

| Methode | Änderung |
|---------|----------|
| `move_selected_to_trash()` | Ruft `store.move_to_trash()` statt `delete_session_file()` + `save_trash()` |
| `restore_selected_from_trash()` | Ruft `store.restore_session()` |
| `empty_trash()` | Ruft `store.empty_trash()` |

### `ui.rs`

| Funktion | Änderung |
|----------|----------|
| `draw_tabs()` | `●`/`○` Marker |
| `draw_commands()` | 2 Zeilen, Kategorien |
| `draw_preview()` | Scroll-Indikator |

---

## Testing

- Unit-Tests in `store.rs`: `move_to_trash`, `restore`, `empty_trash` mit TempDir
- Unit-Tests in `app.rs`: State-Transitions wie bisher
- Snapshot-Tests in `ui.rs`: neue Tab Bar, neue Command Bar, Scroll-Indikator
- Integration-Tests in `tests/integration.rs`: Trash-Roundtrip über echte Dateien
