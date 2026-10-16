/** @type {import('tailwindcss').Config} */
// Couleurs liées aux variables CSS du thème (src/index.css). color-mix permet aux
// modificateurs d'opacité de Tailwind (bg-accent-primary/15…) de fonctionner sur des var().
const themed = (name) => `color-mix(in srgb, var(--${name}) calc(<alpha-value> * 100%), transparent)`;
const tokens = [
  "bg-primary", "bg-secondary", "bg-tertiary", "bg-input", "bg-hover",
  "text-primary", "text-secondary", "text-muted",
  "accent-primary", "accent-secondary", "accent-success", "accent-warning", "accent-error",
  "border-primary",
];

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: Object.fromEntries(tokens.map((t) => [t, themed(t)])),
      fontFamily: { sans: ['"Segoe UI Variable"', '"Segoe UI"', "Inter", "system-ui", "sans-serif"] },
      borderRadius: { win: "8px", "win-lg": "12px" },
      boxShadow: {
        win: "0 2px 8px rgba(0,0,0,0.4), 0 0 0 1px rgba(255,255,255,0.05)",
        "win-hover": "0 4px 16px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.08)",
      },
      animation: { "slide-in": "slideIn 200ms ease-out" },
      keyframes: {
        slideIn: {
          "0%": { opacity: "0", transform: "translateY(-8px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
      },
    },
  },
  plugins: [],
};
