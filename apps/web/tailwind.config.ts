import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "ui-monospace", "monospace"],
        serif: ["Spectral", "Cardo", "ui-serif", "serif"],
      },
      colors: {
        navy: {
          DEFAULT: "#0f1b2d",
          50: "#f5f7fa",
          100: "#e6ecf2",
          200: "#c1cfdc",
          400: "#5b748f",
          600: "#1d2f4b",
          700: "#16263d",
          900: "#0a1322",
        },
        ochre: {
          DEFAULT: "#b15a2b",
          200: "#e8c4af",
          400: "#cb8259",
          600: "#8c451f",
        },
        canvas: "#f8f7f5",
        ink: "#0a1322",
      },
      typography: {
        DEFAULT: {
          css: {
            "code::before": { content: "none" },
            "code::after": { content: "none" },
          },
        },
      },
    },
  },
  plugins: [],
};

export default config;
