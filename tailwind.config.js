/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter", "Segoe UI", "system-ui", "sans-serif"],
      },
      boxShadow: {
        glow: "0 0 48px rgba(52, 211, 153, 0.18)",
      },
    },
  },
  plugins: [],
};
