import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './styles.css';

const root = document.getElementById('root');
if (!root) throw new Error('Root element #root was not found');

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

// Offline support. Service workers require a secure context (HTTPS or
// localhost); on a plain-HTTP address registration is simply skipped and the
// app continues to work online as normal.
if (import.meta.env.PROD && 'serviceWorker' in navigator && window.isSecureContext) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => {
      // Offline support is best-effort; never break the app if it fails.
    });
  });
}
