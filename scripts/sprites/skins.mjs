// Pet skins for the sprite generator. Each skin draws its body for one frame in a
// 128×128 box standing on y = 112; the shared face is drawn on top at `face`.
// p.a is "perkiness" (-1 droopy … 1 perky) and p.t the 0..1 loop phase.

const wave = (t, k = 1) => Math.sin(t * Math.PI * 2 * k);
const deg = (r) => (r * Math.PI) / 180;

function gradient(id, light, dark) {
  return `<radialGradient id="${id}" cx="40%" cy="32%" r="72%"><stop offset="0" stop-color="${light}"/><stop offset="1" stop-color="${dark}"/></radialGradient>`;
}

export const SKINS = {
  blob: {
    label: "Blob",
    defs: gradient("blob-g", "#F4A27E", "#D9643F"),
    face: { eyeY: 68, eyeDX: 16, mouthY: 83 },
    draw: () => `
      <ellipse cx="46" cy="108" rx="10" ry="6" fill="#C4552F"/>
      <ellipse cx="82" cy="108" rx="10" ry="6" fill="#C4552F"/>
      <path d="M64 24c28 0 48 22 48 50 0 26-21 40-48 40S16 100 16 74c0-28 20-50 48-50z" fill="url(#blob-g)"/>`,
  },

  sprout: {
    label: "Sprout",
    defs: gradient("sprout-g", "#FBE3CF", "#EDB896"),
    face: { eyeY: 80, eyeDX: 15, mouthY: 92 },
    // The sprout is the mood antenna: upright when perky, drooping when sad, sways in idle.
    draw: (p) => {
      const droop = (1 - p.a) * 32;
      const sway = wave(p.t) * 6 + (p.a < 0 ? droop * 0.35 : 0);
      return `
      <g transform="rotate(${sway.toFixed(1)} 64 47)">
        <path d="M64 47V29" stroke="#5E9A50" stroke-width="4.5" stroke-linecap="round"/>
        <path d="M64 31c-3-9-12-13-21-11 2 9 11 14 21 11z" fill="#7DB46C" transform="rotate(${(-droop).toFixed(1)} 64 31)"/>
        <path d="M65 28c4-11 15-15 25-12-2 11-14 16-25 12z" fill="#8CC57A" transform="rotate(${droop.toFixed(1)} 65 28)"/>
      </g>
      <path d="M18 98c0-32 20-53 46-53s46 21 46 53c0 9-6 14-15 14H33c-9 0-15-5-15-14z" fill="url(#sprout-g)"/>
      <path d="M30 70c4-10 12-17 22-20" stroke="#fff" stroke-width="4" stroke-linecap="round" fill="none" opacity=".55"/>`;
    },
  },

  beancat: {
    label: "Bean Cat",
    defs: gradient("cat-g", "#F7C88A", "#E0913E"),
    face: { eyeY: 68, eyeDX: 12, mouthY: 80, cat: true },
    // Ears flatten when sad; the tail swishes with the loop phase.
    draw: (p) => {
      const ear = (1 - p.a) * 9;
      const tail = wave(p.t) * 14;
      return `
      <path d="M95 97c15 1 21-17 10-26" stroke="#D07F30" stroke-width="7" stroke-linecap="round" fill="none" transform="rotate(${tail.toFixed(1)} 95 97)"/>
      <g fill="#E0913E" stroke="#E0913E" stroke-width="6" stroke-linejoin="round">
        <path d="M37 50 40 22l19 13z" transform="rotate(${(-ear).toFixed(1)} 44 48)"/>
        <path d="M91 50l-3-28-19 13z" transform="rotate(${ear.toFixed(1)} 84 48)"/>
      </g>
      <path d="M41 45l1.6-14 9 6.4zM87 45l-1.6-14-9 6.4z" fill="#F7A48E" opacity=".8"/>
      <path d="M32 59c0-18 14-28 32-28s32 10 32 28v32c0 14-13 21-32 21s-32-7-32-21z" fill="url(#cat-g)"/>
      <path d="M58 33l6 7 6-7" stroke="#D07F30" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" fill="none" opacity=".6"/>`;
    },
  },

  ember: {
    label: "Ember",
    fit: 0.88,
    defs: `${gradient("ember-g", "#FFC46B", "#F0642A")}
      <radialGradient id="ember-core" cx="50%" cy="60%" r="50%"><stop offset="0" stop-color="#FFE7A8" stop-opacity=".9"/><stop offset="1" stop-color="#FFE7A8" stop-opacity="0"/></radialGradient>`,
    face: { eyeY: 82, eyeDX: 11, mouthY: 94, cheek: "#FF9A7A" },
    // The tip flickers with the phase and leans more when perky; the inner glow dims when low.
    draw: (p) => {
      const flick = wave(p.t, 2) * 4 + p.a * 2;
      const glow = 0.35 + 0.5 * ((p.a + 1) / 2);
      return `
      <path d="M64 112c-24 0-38-15-38-36 0-19 12-31 24-43 7-7 ${11 + flick * 0.3}-15 ${9 + flick}-24 17 5 29 19 32 34 7 8 11 20 11 33 0 21-14 36-38 36z" fill="url(#ember-g)"/>
      <ellipse cx="64" cy="86" rx="26" ry="22" fill="url(#ember-core)" opacity="${glow.toFixed(2)}"/>`;
    },
  },

  // Inspired by Claude's spark mark: clay rays radiating from a round core that holds the face.
  spark: {
    label: "Spark",
    defs: gradient("spark-g", "#E8957A", "#C9603F"),
    face: { eyeY: 66, eyeDX: 10, mouthY: 78, cheek: "#F2B3A0" },
    draw: (p) => {
      const cx = 64;
      const cy = 68;
      const spin = p.t * 30 + wave(p.t) * 3;
      const reach = 1 + 0.12 * p.a;
      const rays = Array.from({ length: 11 }, (_, i) => {
        const ang = deg(i * (360 / 11) + spin + (i % 3) * 4 - 90);
        const len = (i % 2 ? 37 : 44) * reach + (i % 3) * 2;
        // Droop pulls the rays toward the ground when sad or sleepy.
        const sag = p.a < 0 ? -p.a * 7 * Math.max(0, Math.cos(ang)) ** 2 : 0;
        const x2 = cx + Math.cos(ang) * len;
        const y2 = cy + Math.sin(ang) * len + sag;
        return `<path d="M${cx} ${cy}L${x2.toFixed(1)} ${y2.toFixed(1)}"/>`;
      }).join("");
      return `
      <g stroke="#D97757" stroke-width="10" stroke-linecap="round">${rays}</g>
      <circle cx="${cx}" cy="${cy}" r="27" fill="url(#spark-g)"/>`;
    },
  },
};
