import React from 'react';
import ReactDOM from 'react-dom/client';
import { HashRouter } from 'react-router-dom';
import { QueryProvider } from '@/app/providers/QueryProvider';
import { ThemeProvider } from '@/app/providers/ThemeProvider';
import { I18nProvider } from '@/shared/i18n/provider';
import { AppRoutes } from '@/app/router';
import '@/styles/index.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryProvider>
      <ThemeProvider>
        <I18nProvider>
        <HashRouter>
          <AppRoutes />
        </HashRouter>
        </I18nProvider>
      </ThemeProvider>
    </QueryProvider>
  </React.StrictMode>,
);
