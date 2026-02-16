/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        brain: {
          bg: "#1a1b1e",
          surface: "#25262b",
          border: "#373a40",
          text: "#c1c2c5",
          accent: "#7c5cfc",
          success: "#40c057",
          warning: "#fab005",
          error: "#fa5252",
        },
      },
      animation: {
        "slide-down": "slideDown 150ms ease-out",
        "fade-in": "fadeIn 100ms ease-out",
      },
      keyframes: {
        slideDown: {
          from: { transform: "translateY(-10px)", opacity: "0" },
          to: { transform: "translateY(0)", opacity: "1" },
        },
        fadeIn: {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
      },
    },
  },
  plugins: [],
};
