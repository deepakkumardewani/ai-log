/** Tailwind CSS v4 configuration for cclog.
 *  Material Design 3 dark theme.
 *  Used by the standalone Tailwind CLI at build time.
 *  DO NOT reference external CDNs — output must be fully self-contained.
 */
export default {
  content: [],
  theme: {
    extend: {
      colors: {
        surface: '#0A0A0A',
        'surface-container': '#1A1A1A',
        'surface-container-high': '#242424',
        'surface-container-highest': '#2E2E2E',
        'on-surface': '#E0E0E0',
        'on-surface-variant': '#A0A0A0',
        primary: '#7C4DFF',
        'on-primary': '#FFFFFF',
        'primary-container': '#3700B3',
        'on-primary-container': '#D4BFF9',
        secondary: '#03DAC6',
        'on-secondary': '#000000',
        'secondary-container': '#004D40',
        'on-secondary-container': '#A7F3D0',
        error: '#CF6679',
        'on-error': '#000000',
        'error-container': '#B00020',
        'on-error-container': '#FCD8DF',
        outline: '#3D3D3D',
        border: '#262626',
        background: '#0A0A0A',
      },
      fontFamily: {
        sans: ['Geist', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
        display: ['Space Grotesk', 'sans-serif'],
      },
    },
  },
};
