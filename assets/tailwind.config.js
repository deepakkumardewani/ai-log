/** Tailwind CSS v4 configuration for cclog.
 *  Warm neutral + terracotta/amber palette.
 *  Used by the standalone Tailwind CLI at build time.
 *  DO NOT reference external CDNs — output must be fully self-contained.
 */
export default {
  content: [],
  theme: {
    extend: {
      colors: {
        // Surface tokens — warm neutrals
        surface: '#161410',
        'surface-elevated': '#1F1C18',
        'surface-hover': '#28241F',
        'surface-pressed': '#322D27',
        // Text tokens
        'text-primary': '#EBE5DD',
        'text-secondary': '#A0998F',
        'text-tertiary': '#6B6660',
        // Accent — terracotta
        accent: '#D4683A',
        'accent-hover': '#E07B50',
        'text-accent': '#E8926C',
        'ring-accent': '#E8926C',
        // Borders
        'border-subtle': '#2A2621',
        'border-default': '#3D3832',
        // Semantic state
        success: '#5BAD75',
        error: '#D9665E',
        warning: '#E2A04A',
        // Role indicator colors
        'state-user': '#D4852B',
        'state-assistant': '#D4683A',
        'state-thinking': '#8B7EC8',
        'state-tool': '#4A9E8E',
        'state-system': '#8B8680',
      },
      fontFamily: {
        sans: ['Geist', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
        display: ['Space Grotesk', 'sans-serif'],
      },
    },
  },
};
