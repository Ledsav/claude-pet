import type { Pet } from "./api";

type Stat = "hunger" | "mood" | "energy";
const STATS: { key: Stat; label: string }[] = [
  { key: "hunger", label: "food" },
  { key: "mood", label: "mood" },
  { key: "energy", label: "energy" },
];

export type PanelAction = "feed" | "play" | "sleep";

/** Stat bars, action buttons and the chat box shown while hovering the pet. */
export class Panel {
  private bars = new Map<Stat, { fill: HTMLElement; value: HTMLElement }>();
  private sleepBtn: HTMLButtonElement;
  private input: HTMLInputElement;
  private sendBtn: HTMLButtonElement;

  constructor(
    el: HTMLElement,
    onAction: (a: PanelAction) => void,
    onChat: (msg: string) => void,
  ) {
    el.innerHTML = /* html */ `
      <div class="stats">
        ${STATS.map(
          (s) => `
          <div class="stat" data-stat="${s.key}">
            <span class="stat-label">${s.label}</span>
            <span class="stat-track"><span class="stat-fill"></span></span>
            <span class="stat-value">0</span>
          </div>`,
        ).join("")}
      </div>
      <div class="actions">
        <button type="button" data-action="feed">feed</button>
        <button type="button" data-action="play">play</button>
        <button type="button" data-action="sleep">sleep</button>
      </div>
      <form class="chat" autocomplete="off">
        <input name="msg" maxlength="500" placeholder="say something… or /help" aria-label="Message to your pet" />
        <button type="submit" aria-label="Send">↑</button>
      </form>`;

    for (const s of STATS) {
      const row = el.querySelector<HTMLElement>(`[data-stat="${s.key}"]`)!;
      this.bars.set(s.key, {
        fill: row.querySelector(".stat-fill")!,
        value: row.querySelector(".stat-value")!,
      });
    }

    el.querySelectorAll<HTMLButtonElement>("[data-action]").forEach((btn) =>
      btn.addEventListener("click", () => onAction(btn.dataset.action as PanelAction)),
    );
    this.sleepBtn = el.querySelector('[data-action="sleep"]')!;

    const form = el.querySelector<HTMLFormElement>(".chat")!;
    this.input = form.querySelector("input")!;
    this.sendBtn = form.querySelector("button")!;
    form.addEventListener("submit", (e) => {
      e.preventDefault();
      const msg = this.input.value.trim();
      if (!msg || this.sendBtn.disabled) return;
      this.input.value = "";
      onChat(msg);
    });
  }

  update(pet: Pet) {
    for (const { key } of STATS) {
      const v = Math.round(pet[key]);
      const bar = this.bars.get(key)!;
      bar.fill.style.width = `${v}%`;
      bar.fill.dataset.level = v < 25 ? "bad" : v < 55 ? "warn" : "good";
      bar.value.textContent = String(v);
    }
    this.sleepBtn.textContent = pet.asleep ? "wake" : "sleep";
  }

  setThinking(thinking: boolean) {
    this.sendBtn.disabled = thinking;
  }
}
