/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      animation: {
        "orb-pulse": "orbPulse 2s ease-in-out infinite",
        "orb-breathe": "orbBreathe 3s ease-in-out infinite",
        "waveform": "waveform 1.2s ease-in-out infinite",
        "fade-in": "fadeIn 0.3s ease-out",
        "slide-up": "slideUp 0.4s cubic-bezier(0.16, 1, 0.3, 1)",
      },
      keyframes: {
        orbPulse: {
          "0%, 100%": { transform: "scale(1) translateZ(0)", opacity: "0.85" },
          "50%": { transform: "scale(1.12) translateZ(0)", opacity: "1" },
        },
        orbBreathe: {
          "0%, 100%": { transform: "scale(1) translateZ(0)", opacity: "0.88" },
          "50%": { transform: "scale(1.09) translateZ(0)", opacity: "1" },
        },
        waveform: {
          "0%, 100%": { transform: "scaleY(0.4)" },
          "50%": { transform: "scaleY(1)" },
        },
        fadeIn: {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        fadeSlideIn: {
          from: { opacity: "0", transform: "translateY(3px)" },
          to:   { opacity: "1", transform: "translateY(0)" },
        },
        pulse: {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.4" },
        },
        slideUp: {
          from: { transform: "translateY(20px)", opacity: "0" },
          to: { transform: "translateY(0)", opacity: "1" },
        },
      },
    },
  },
  plugins: [],
};
