import * as fs from "fs";
import * as path from "path";
import * as os from "os";

export function createTempEnv() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "asm-e2e-"));
  const claudeDir = path.join(tmp, "claude");
  const configDir = path.join(tmp, "config");
  const exportDir = path.join(tmp, "exports");
  fs.mkdirSync(claudeDir, { recursive: true });
  fs.mkdirSync(configDir, { recursive: true });
  fs.mkdirSync(exportDir, { recursive: true });
  return { tmp, claudeDir, configDir, exportDir };
}

export function createFixtureSession(
  claudeDir: string,
  projectSlug: string,
  sessionId: string,
  messages: Array<[string, string]>
) {
  const sessionsDir = path.join(claudeDir, "projects", projectSlug);
  fs.mkdirSync(sessionsDir, { recursive: true });
  const lines = messages.map(([role, content], i) => {
    const type_ = role === "user" ? "user" : "assistant";
    const contentJson =
      role === "assistant"
        ? `[{"type":"text","text":"${content}"}]`
        : `"${content}"`;
    return `{"type":"${type_}","message":{"role":"${role}","content":${contentJson}},"uuid":"test-uuid-${String(i).padStart(3, "0")}"}`;
  });
  fs.writeFileSync(
    path.join(sessionsDir, `${sessionId}.jsonl`),
    lines.join("\n")
  );
}

export function cleanup(tmp: string) {
  fs.rmSync(tmp, { recursive: true, force: true });
}
