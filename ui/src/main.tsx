import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './frontend/themes/reset.css';
import './frontend/themes/tokens.css';
import './frontend/themes/components.css';
import './frontend/themes/responsive.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
