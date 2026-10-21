import { PetView } from "./pet";

const bubbleEl = document.getElementById("bubble")!;
let bubbleTimer: number | undefined;

/** Shows a speech bubble; it stays up longer for longer text. */
export function say(text: string, ms = Math.min(12000, 2500 + text.length * 60)) {
  bubbleEl.textContent = text;
  bubbleEl.classList.add("show");
  clearTimeout(bubbleTimer);
  if (ms > 0) bubbleTimer = window.setTimeout(() => bubbleEl.classList.remove("show"), ms);
}

const pet = new PetView(document.getElementById("pet")!, () => {
  pet.play("wiggle", 900);
});

say("hi! i'm mochi.");
