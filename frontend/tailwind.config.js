/** @type {import('tailwindcss').Config} */
export default {
    content: [
      "./index.html",
      "./src/**/*.{js,ts,jsx,tsx}",
    ],
    theme: {
      extend: {
        colors: {
          background: "#09090b", 
          surface: "#18181b",    
          border: "#27272a",     
          primary: "#22c55e",    
          danger: "#ef4444",     
          accent: "#3b82f6",     
        },
        fontFamily: {
          mono: ['"JetBrains Mono"', 'monospace', 'ui-monospace', 'SFMono-Regular'],
        }
      },
    },
    plugins: [],
  }