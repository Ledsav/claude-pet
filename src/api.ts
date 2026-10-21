import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Mirrors `stats::Pet` in Rust. All stats are 0–100, higher is better. */
export type Pet = {
  hunger: number;
  mood: number;
  energy: number;
  asleep: boolean;
  updated_at: number;
};

/** Mirrors `stats::Outcome`: the result of an action, from the UI or the tray. */
export type Reaction = {
  action: "feed" | "play" | "sleep" | "wake";
  ok: boolean;
  message: string;
};

export const getPet = () => invoke<Pet>("get_pet");
export const act = (action: "feed" | "play" | "sleep") => invoke<Reaction>(action);
export const chat = (message: string) => invoke<string>("chat", { message });

export const onStats = (cb: (pet: Pet) => void) => listen<Pet>("stats", (e) => cb(e.payload));
export const onReaction = (cb: (r: Reaction) => void) =>
  listen<Reaction>("reaction", (e) => cb(e.payload));
