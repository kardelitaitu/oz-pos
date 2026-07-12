import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './frontend/themes/reset.css';
import './frontend/themes/tokens.css';
import './frontend/themes/components.css';
import './frontend/themes/responsive.css';

localStorage.clear();
console.log("[ThemeDebug] localStorage cleared on startup.");

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
