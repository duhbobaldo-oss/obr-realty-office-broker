import { useEffect, useState } from 'react';
import { getDisplayCurrency, getUiLanguage, onLocaleChange, t, tf, type GccCurrencyCode, type UiLanguage } from '../domain/locale';

/** يعيد رسم المكوّن عند تغيّر اللغة أو العملة */
export function useLocale() {
  const [lang, setLang] = useState<UiLanguage>(() => getUiLanguage());
  const [currency, setCurrency] = useState<GccCurrencyCode>(() => getDisplayCurrency());
  const [, setTick] = useState(0);

  useEffect(() => {
    return onLocaleChange(() => {
      setLang(getUiLanguage());
      setCurrency(getDisplayCurrency());
      setTick((n) => n + 1);
    });
  }, []);

  return { lang, currency, t, tf, isEn: lang === 'en' };
}
