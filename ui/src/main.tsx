import React from 'react';
import ReactDOM from 'react-dom/client';
import { LocalizationProvider } from '@fluent/react';
import { createEnUsLocalization } from './locales';
import App from './App';
import './frontend/themes/reset.css';
import './frontend/themes/tokens.css';
import './frontend/themes/components.css';

const l10n = createEnUsLocalization();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <LocalizationProvider l10n={l10n}>
      <App />
    </LocalizationProvider>
  </React.StrictMode>,
);
