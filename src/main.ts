import {
  act,
  chat,
  getPet,
  getSettings,
  onNudge,
  onReaction,
  onSettings,
  onStats,
  openPersonality,
  quit,
  runCommand,
  type Pet,
  type Reaction,
} from "./api";
import { listen } from "@tauri-apps/api/event";
import { Menu } from "@tauri-apps/api/menu";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Panel } from "./panel";
import { PetView, type Mood } from "./pet";

const bubbleEl = document.getElementById("bubble")!;
const bubbleText = bubbleEl.querySelector<HTMLElement>(".bubble-text")!;
let bubbleTimer: number | undefined;

/**
 * Shows a speech bubble for long enough to read it (about 15 characters a second).
 * It stays up while hovered, so long replies can be scrolled. `ms = 0` keeps it up.
 */
function say(text: string, ms = Math.min(30000, 3000 + text.length * 65)) {
  bubbleEl.classList.remove("thinking");
  bubbleText.textContent = text;
  bubbleText.scrollTop = 0;
  bubbleEl.classList.add("show");
  clearTimeout(bubbleTimer);
  if (ms > 0) bubbleTimer = window.setTimeout(hideBubble, ms);
}

function hideBubble() {
  if (bubbleEl.matches(":hover")) {
    bubbleTimer = window.setTimeout(hideBubble, 1500);
    return;
  }
  bubbleEl.classList.remove("show");
}

/** The pet "speaks" a line Claude wrote. */
function speak(text: string) {
  pet.play("talk", Math.min(3000, 400 + text.length * 25));
  say(text);
}

function moodOf(p: Pet): Mood {
  if (p.asleep) return "sleeping";
  if (p.hunger < 25) return "hungry";
  if (p.energy < 20) return "tired";
  if (p.mood < 30) return "sad";
  if (p.mood > 70) return "happy";
  return "neutral";
}

const pet = new PetView(document.getElementById("pet")!, () => pet.play("wiggle"));
const panel = new Panel(
  document.getElementById("panel")!,
  (action) => void act(action),
  (msg) => void talk(msg),
);

async function talk(msg: string) {
  if (/^\/(bye|quit|exit)\s*$/i.test(msg)) return farewell();
  if (msg.startsWith("/")) return command(msg);
  panel.setThinking(true);
  bubbleText.textContent = "";
  bubbleEl.classList.add("show", "thinking");
  clearTimeout(bubbleTimer);
  try {
    speak(await chat(msg));
  } catch (err) {
    pet.play("wiggle");
    say(String(err));
  } finally {
    panel.setThinking(false);
  }
}

/** Slash commands are handled by the app itself, never sent to Claude. */
async function command(input: string) {
  const pep = /^\/(motivate|pep)\b/i.test(input);
  try {
    const out = await runCommand(input);
    if (pep) speak(out);
    else say(out, 10000);
  } catch (err) {
    say(String(err), 6000);
  }
}

let leaving = false;

/** Goodbye: the pet waves, flops, its little soul floats away, then the app closes. */
async function farewell() {
  if (leaving) return;
  leaving = true;
  document.body.classList.add("leaving");
  say("bye bye! see you soon", 0);
  await pet.playFinal("bye");
  await quit();
}

let latest: Pet | undefined;

/** Right-click on the pet: the everyday actions, plus a way out. */
async function showPetMenu() {
  if (leaving) return;
  const menu = await Menu.new({
    items: [
      { id: "feed", text: "Feed", action: () => void act("feed") },
      { id: "play", text: "Play", action: () => void act("play") },
      { id: "sleep", text: latest?.asleep ? "Wake up" : "Sleep", action: () => void act("sleep") },
      { item: "Separator" },
      { id: "personality", text: "Edit personality…", action: () => void openPersonality() },
      { id: "hide", text: "Hide (tray icon brings me back)", action: () => void getCurrentWindow().hide() },
      { item: "Separator" },
      { id: "bye", text: "Bye bye 👋", action: () => void farewell() },
    ],
  });
  await menu.popup();
}

document.addEventListener("contextmenu", (e) => {
  // Keep the browser's menu for the text fields; everything else gets ours.
  if ((e.target as HTMLElement).closest("input, textarea")) return;
  e.preventDefault();
  if ((e.target as HTMLElement).closest("#pet")) void showPetMenu();
});

function render(p: Pet) {
  latest = p;
  pet.setMood(moodOf(p));
  panel.update(p);
}

const ANIM: Partial<Record<Reaction["action"], "eat" | "play">> = { feed: "eat", play: "play" };

function react(r: Reaction) {
  const anim = r.ok ? ANIM[r.action] : undefined;
  if (anim) pet.play(anim);
  else if (!r.ok) pet.play("wiggle");
  say(r.message);
}

void onStats(render);
void onReaction(react);
void onSettings((s) => pet.setSkin(s.skin));
void onNudge(speak);
void listen("farewell", () => void farewell()); // Quit from the tray
void Promise.all([getPet(), getSettings()]).then(([p, s]) => {
  pet.setSkin(s.skin);
  render(p);
  // First-run help for colleagues who don't have Claude Code yet.
  runCommand("/doctor").catch((err) => say(String(err), 0));
  if (p.asleep) say("zzz…");
  else if (s.skin === "spine") say("oh good, it's you. spiny's here, try to contain yourself. /help if you must");
  else say(`hi! i'm ${s.name.toLowerCase()}. type /help for commands`);
});
