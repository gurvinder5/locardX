import { heroui } from '@heroui/react';

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './index.html',
    './src/**/*.{js,ts,jsx,tsx}',
    './node_modules/@heroui/theme/dist/**/*.{js,ts,jsx,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        workstation: {
          bg: '#0B0F14',
          sidebar: '#10161D',
          panel: '#151C24',
          surface: '#1B2530',
          elevated: '#1B2530',
          border: '#26313D',
          hover: '#1B2530',
          'active-nav': '#26313D',
          primary: '#E5E7EB',
          secondary: '#929EAC',
        },
      },
    },
  },
  darkMode: 'class',
  plugins: [
    heroui({
      themes: {
        light: {
          colors: {
            background: '#F8FAFC',
            foreground: '#111827',
            content1: '#FFFFFF',
            content2: '#F1F5F9',
            content3: '#E2E8F0',
            content4: '#CBD5E1',
            divider: '#E2E8F0',
            focus: '#64748B',
            default: {
              DEFAULT: '#F1F5F9',
              foreground: '#111827',
            },
            primary: {
              DEFAULT: '#0F172A',
              foreground: '#FFFFFF',
            },
            danger: {
              DEFAULT: '#BE123C',
              foreground: '#FFFFFF',
            },
            warning: {
              DEFAULT: '#D97706',
              foreground: '#FFFFFF',
            },
          },
        },
        dark: {
          colors: {
            background: '#0B0F14',
            foreground: '#E5E7EB',
            content1: '#151C24',
            content2: '#1B2530',
            content3: '#26313D',
            content4: '#344252',
            divider: '#26313D',
            focus: '#929EAC',
            default: {
              DEFAULT: '#1B2530',
              foreground: '#E5E7EB',
            },
            primary: {
              DEFAULT: '#E5E7EB',
              foreground: '#0B0F14',
            },
            danger: {
              DEFAULT: '#BE123C',
              foreground: '#FFFFFF',
            },
            warning: {
              DEFAULT: '#D97706',
              foreground: '#FFFFFF',
            },
          },
        },
      },
    }),
  ],
};
