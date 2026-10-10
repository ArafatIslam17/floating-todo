import '@fontsource-variable/manrope';
import './styles/app.css';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { isTauri } from './lib/backend';

if (!isTauri()) document.documentElement.classList.add('preview');

// The desktop widget has no use for the browser context menu (and it exposes "Reload").
if (isTauri() && !import.meta.env.DEV) {
  window.addEventListener('contextmenu', (e) => {
    if (!(e.target instanceof HTMLElement && e.target.matches('input, textarea'))) e.preventDefault();
  });
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
