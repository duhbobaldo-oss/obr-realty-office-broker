import { Route, Routes } from 'react-router-dom';
import { useEffect, useState } from 'react';
import Layout from './components/Layout';
import Dashboard from './pages/Dashboard';
import Properties from './pages/Properties';
import PropertyForm from './pages/PropertyForm';
import Clients from './pages/Clients';
import ClientForm from './pages/ClientForm';
import Market from './pages/Market';
import Campaigns from './pages/Campaigns';
import Finance from './pages/Finance';
import Brochures from './pages/Brochures';
import Settings from './pages/Settings';
import OfferDesign from './pages/OfferDesign';
import PropertyMap from './pages/PropertyMap';
import Help from './pages/Help';
import DatabasePage from './pages/DatabasePage';
import { initTheme } from './theme';
import { applyDocumentLocale, getUiLanguage, onLocaleChange, t } from './domain/locale';
import { api } from './api';

export interface AuthState {
  token: string;
  user: any;
}

export default function App() {
  const [auth, setAuth] = useState<AuthState | null>(null);
  const [loading, setLoading] = useState(true);
  const [bootError, setBootError] = useState('');
  const [uiLang, setUiLang] = useState(() => getUiLanguage());

  useEffect(() => {
    initTheme();
    applyDocumentLocale();
    return onLocaleChange(() => setUiLang(getUiLanguage()));
  }, []);

  useEffect(() => {
    const bootstrap = async () => {
      try {
        const saved = localStorage.getItem('mawasib_auth');
        if (saved) {
          try {
            const parsed = JSON.parse(saved) as AuthState;
            if (parsed?.token) {
              localStorage.setItem('mawasib_auth', JSON.stringify(parsed));
              const current = await api<any>('/api/auth/me');
              const refreshed = { ...parsed, user: current.user };
              localStorage.setItem('mawasib_auth', JSON.stringify(refreshed));
              setAuth(refreshed);
              return;
            }
          } catch {
            localStorage.removeItem('mawasib_auth');
          }
        }

        const data = await api('/api/auth/ensure-local', {
          method: 'POST',
          auth: false
        });
        localStorage.setItem('mawasib_auth', JSON.stringify(data));
        setAuth(data);
      } catch (err: any) {
        setBootError(err?.message || t('boot-failed'));
      } finally {
        setLoading(false);
      }
    };

    bootstrap();
  }, []);

  if (loading) {
    return <div className="auth-page"><div className="auth-card">{t('loading')}</div></div>;
  }

  if (!auth) {
    return (
      <div className="auth-page">
        <div className="auth-card">
          <p style={{ color: 'var(--danger)', margin: 0 }}>{bootError || t('boot-failed')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app-shell" key={uiLang}>
      <Routes>
        <Route
          path="/*"
          element={
            <Layout auth={auth}>
              <Routes>
                <Route path="/" element={<Dashboard />} />
                <Route path="/properties" element={<Properties />} />
                <Route path="/properties/new" element={<PropertyForm />} />
                <Route path="/properties/:id" element={<PropertyForm />} />
                <Route path="/maps" element={<PropertyMap />} />
                <Route path="/clients" element={<Clients />} />
                <Route path="/clients/new" element={<ClientForm />} />
                <Route path="/clients/:id" element={<ClientForm />} />
                <Route path="/market" element={<Market />} />
                <Route path="/campaigns" element={<Campaigns />} />
                <Route path="/finance" element={<Finance />} />
                <Route path="/brochures" element={<Brochures />} />
                <Route path="/offer-design" element={<OfferDesign />} />
                <Route path="/database" element={<DatabasePage />} />
                <Route path="/settings" element={<Settings auth={auth} />} />
                <Route path="/help" element={<Help />} />
              </Routes>
            </Layout>
          }
        />
      </Routes>
    </div>
  );
}
