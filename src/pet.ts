import { getCurrentWindow } from "@tauri-apps/api/window";

/** Visual states the pet can be in. Exactly one "mood" plus optional one-shot animations. */
export type Mood = "happy" | "neutral" | "sad" | "hungry" | "tired" | "sleeping";
export type Anim = "wiggle" | "eat" | "play" | "talk";

const SVG = /* html */ `
<svg viewBox="0 0 120 120" class="pet-svg" aria-hidden="true">
  <defs>
    <radialGradient id="body" cx="40%" cy="35%" r="70%">
      <stop offset="0" stop-color="#F4A27E"/>
      <stop offset="1" stop-color="#D9643F"/>
    </radialGradient>
  </defs>
  <ellipse class="shadow" cx="60" cy="112" rx="34" ry="5"/>
  <g class="body-group">
    <ellipse class="foot" cx="42" cy="104" rx="10" ry="6"/>
    <ellipse class="foot" cx="78" cy="104" rx="10" ry="6"/>
    <path class="body" fill="url(#body)"
      d="M60 18c28 0 48 22 48 50 0 26-21 40-48 40S12 94 12 68c0-28 20-50 48-50z"/>
    <g class="eyes eyes-open">
      <g class="eye"><ellipse cx="44" cy="62" rx="7" ry="9"/><circle class="glint" cx="46.5" cy="58.5" r="2.5"/></g>
      <g class="eye"><ellipse cx="76" cy="62" rx="7" ry="9"/><circle class="glint" cx="78.5" cy="58.5" r="2.5"/></g>
    </g>
    <g class="eyes eyes-closed">
      <path d="M37 63q7 6 14 0"/><path d="M69 63q7 6 14 0"/>
    </g>
    <g class="eyes eyes-tired">
      <path d="M37 64h14"/><path d="M69 64h14"/>
    </g>
    <ellipse class="cheek" cx="32" cy="76" rx="7" ry="4"/>
    <ellipse class="cheek" cx="88" cy="76" rx="7" ry="4"/>
    <path class="mouth mouth-smile" d="M53 78q7 7 14 0"/>
    <path class="mouth mouth-flat" d="M54 80h12"/>
    <path class="mouth mouth-frown" d="M53 83q7 -6 14 0"/>
    <ellipse class="mouth mouth-open" cx="60" cy="80" rx="5" ry="4"/>
  </g>
  <g class="zzz"><text x="92" y="30">z</text><text x="100" y="18">z</text></g>
  <path class="heart" d="M60 14c-3-5-11-4-11 2 0 5 11 11 11 11s11-6 11-11c0-6-8-7-11-2z"/>
</svg>`;

export class PetView {
  private anim?: number;

  constructor(private el: HTMLElement, onClick: () => void) {
    el.innerHTML = SVG;
    this.setMood("neutral");
    this.enableDrag(onClick);
  }

  setMood(mood: Mood) {
    this.el.dataset.mood = mood;
  }

  /** Plays a one-shot animation; a new one replaces whatever is running. */
  play(anim: Anim, ms = 1200) {
    this.el.dataset.anim = anim;
    clearTimeout(this.anim);
    this.anim = window.setTimeout(() => delete this.el.dataset.anim, ms);
  }

  /** A press that moves more than a few pixels drags the window; otherwise it's a click. */
  private enableDrag(onClick: () => void) {
    let start: { x: number; y: number } | null = null;

    this.el.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      start = { x: e.screenX, y: e.screenY };
    });
    this.el.addEventListener("pointermove", (e) => {
      if (!start) return;
      if (Math.hypot(e.screenX - start.x, e.screenY - start.y) > 4) {
        start = null;
        void getCurrentWindow().startDragging();
      }
    });
    this.el.addEventListener("pointerup", () => {
      if (start) onClick();
      start = null;
    });
  }
}
