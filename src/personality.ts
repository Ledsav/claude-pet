import { invoke } from "@tauri-apps/api/core";
import { getSettings } from "./api";

const text = document.getElementById("text") as HTMLTextAreaElement;
const resetBtn = document.getElementById("reset") as HTMLButtonElement;
const openBtn = document.getElementById("open-file") as HTMLButtonElement;
const status = document.getElementById("status")!;

let saved = "";
let statusTimer: number | undefined;
let saveTimer: number | undefined;
const AUTOSAVE_MS = 700;

const dirty = () => text.value !== saved;

function setStatus(msg: string, kind: "info" | "error" = "info", ms = 2500) {
  status.textContent = msg;
  status.dataset.kind = kind;
  clearTimeout(statusTimer);
  if (ms > 0) statusTimer = window.setTimeout(refresh, ms);
}

function refresh() {
  status.dataset.kind = "info";
  status.textContent = dirty() ? "Saving…" : "";
  document.title = dirty() ? "• Personality" : "Personality";
}

async function load(value?: string) {
  saved = value ?? (await invoke<string>("get_personality"));
  text.value = saved;
  refresh();
}

/**
 * Saves now. Edits are also saved automatically shortly after typing stops, so closing
 * the window never has to wait for anything (or block on it).
 */
async function save() {
  clearTimeout(saveTimer);
  if (!dirty()) return true;
  try {
    await invoke("save_personality", { text: text.value });
    saved = text.value;
    refresh();
    setStatus("Saved. Applies on the next chat");
    return true;
  } catch (err) {
    setStatus(String(err), "error", 0);
    return false;
  }
}

// Reset is destructive, so it takes a second click within a few seconds.
let resetArmed: number | undefined;
resetBtn.addEventListener("click", async () => {
  if (resetArmed === undefined) {
    resetBtn.textContent = "Click again to reset";
    resetBtn.classList.add("danger");
    resetArmed = window.setTimeout(disarmReset, 3000);
    return;
  }
  disarmReset();
  try {
    await load(await invoke<string>("reset_personality"));
    setStatus("Restored the default personality");
  } catch (err) {
    setStatus(String(err), "error", 0);
  }
});

function disarmReset() {
  clearTimeout(resetArmed);
  resetArmed = undefined;
  resetBtn.textContent = "Reset to default";
  resetBtn.classList.remove("danger");
}

openBtn.addEventListener("click", async () => {
  if (!(await save())) return;
  invoke("open_personality_file").catch((err) => setStatus(String(err), "error", 0));
});

text.addEventListener("input", () => {
  refresh();
  clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => void save(), AUTOSAVE_MS);
});
// Leaving the window flushes a pending save right away.
window.addEventListener("blur", () => void save());
document.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
    e.preventDefault();
    void save();
  }
});

// Reload if the file was edited elsewhere while this window was in the background.
window.addEventListener("focus", async () => {
  if (dirty()) return;
  const onDisk = await invoke<string>("get_personality").catch(() => saved);
  if (onDisk !== saved) await load(onDisk);
});

void getSettings().then((s) => {
  document.getElementById("title")!.textContent = `${s.name}'s personality`;
});
load().catch((err) => setStatus(String(err), "error", 0));
text.focus();
