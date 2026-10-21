import { act, getPet, onReaction, onStats, type Pet, type Reaction } from "./api";
import { Panel } from "./panel";
import { PetView, type Mood } from "./pet";

const bubbleEl = document.getElementById("bubble")!;
let bubbleTimer: number | undefined;

/** Shows a speech bubble; it stays up longer for longer text. `ms = 0` keeps it up. */
function say(text: string, ms = Math.min(12000, 2500 + text.length * 60)) {
  bubbleEl.classList.remove("thinking");
  bubbleEl.textContent = text;
  bubbleEl.classList.add("show");
  clearTimeout(bubbleTimer);
  if (ms > 0) bubbleTimer = window.setTimeout(() => bubbleEl.classList.remove("show"), ms);
}

function moodOf(p: Pet): Mood {
  if (p.asleep) return "sleeping";
  if (p.hunger < 25) return "hungry";
  if (p.energy < 20) return "tired";
  if (p.mood < 30) return "sad";
  if (p.mood > 70) return "happy";
  return "neutral";
}

const pet = new PetView(document.getElementById("pet")!, () => pet.play("wiggle", 900));
const panel = new Panel(
  document.getElementById("panel")!,
  (action) => void act(action),
  () => {},
);

function render(p: Pet) {
  pet.setMood(moodOf(p));
  panel.update(p);
}

const ANIM: Partial<Record<Reaction["action"], "eat" | "play">> = { feed: "eat", play: "play" };

function react(r: Reaction) {
  const anim = r.ok ? ANIM[r.action] : undefined;
  if (anim) pet.play(anim, 1300);
  else if (!r.ok) pet.play("wiggle", 600);
  say(r.message);
}

void onStats(render);
void onReaction(react);
void getPet().then((p) => {
  render(p);
  say(p.asleep ? "zzz…" : "hi! i'm mochi.");
});
