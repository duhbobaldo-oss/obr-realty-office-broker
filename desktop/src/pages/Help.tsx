import React from 'react';
import {
  Rocket, MessageSquare, ShieldCheck, HardDriveDownload, CircleHelp, Lightbulb, LayoutTemplate, Database,
  LayoutDashboard, TrendingUp, Building2, MapPin, Users, Megaphone, Banknote, FolderOpen, PenTool, Settings, BookOpen, Layers
} from 'lucide-react';
import { SECTION_GUIDES } from '../domain/sectionGuides';
import { BUILT_IN_TEMPLATES } from '../domain/builtInTemplates';
import { useLocale } from '../hooks/useLocale';
import type { LucideIcon } from 'lucide-react';

type Translate = (key: string) => string;

const GUIDE_ICONS: Record<string, LucideIcon> = {
  dashboard: LayoutDashboard,
  market: TrendingUp,
  properties: Building2,
  maps: MapPin,
  clients: Users,
  campaigns: Megaphone,
  finance: Banknote,
  brochures: FolderOpen,
  'offer-design': PenTool,
  database: Database,
  settings: Settings,
  help: CircleHelp
};

function extraTopics(t: Translate) {
  const templateSamples = BUILT_IN_TEMPLATES.slice(0, 3).map((tpl) => {
    const kindKey =
      tpl.kind === 'offer_print'
        ? 'help.tpl.kind.offer_print'
        : tpl.kind === 'brochure'
          ? 'help.tpl.kind.brochure'
          : `help.tpl.kind.${tpl.kind}`;
    const nameKey = `help.tpl.${tpl.id}`;
    return `${t(kindKey)}: ${t(nameKey)}`;
  });

  return [
    {
      id: 'start',
      icon: <Rocket size={20} strokeWidth={1.8} />,
      title: t('help.extra.start'),
      items: [t('help.extra.start.1'), t('help.extra.start.2'), t('help.extra.start.3')]
    },
    {
      id: 'templates',
      icon: <LayoutTemplate size={20} strokeWidth={1.8} />,
      title: t('help.extra.templates'),
      items: [t('help.extra.templates.1'), t('help.extra.templates.2'), ...templateSamples]
    },
    {
      id: 'training',
      icon: <Database size={20} strokeWidth={1.8} />,
      title: t('help.extra.training'),
      items: [t('help.extra.training.1'), t('help.extra.training.2')]
    },
    {
      id: 'whatsapp',
      icon: <MessageSquare size={20} strokeWidth={1.8} />,
      title: t('help.extra.whatsapp'),
      items: [t('help.extra.whatsapp.1'), t('help.extra.whatsapp.2'), t('help.extra.whatsapp.3')]
    },
    {
      id: 'backup',
      icon: <HardDriveDownload size={20} strokeWidth={1.8} />,
      title: t('help.extra.backup'),
      items: [t('help.extra.backup.1'), t('help.extra.backup.2')]
    },
    {
      id: 'privacy',
      icon: <ShieldCheck size={20} strokeWidth={1.8} />,
      title: t('help.extra.privacy'),
      items: [t('help.extra.privacy.1'), t('help.extra.privacy.2')]
    }
  ];
}

export default function Help() {
  const { t } = useLocale();
  const topics = extraTopics(t);

  return (
    <div>
      <section className="content-card"><h2>تخصيص OBR حسب الدولة</h2><p>يعمل OBR وفق الدولة المحددة من الإعدادات. عند اختيار الدولة يتم ضبط العملة، أنواع التراخيص العقارية، الروابط الرسمية للخدمات العقارية، ومنصات التسويق المناسبة تلقائيًا. تحتفظ السجلات السابقة بالدولة والعملة التي أُنشئت بها، بينما تستخدم السجلات الجديدة إعداد الدولة الحالي.</p><p>السجل المرتبط بسجل سابق يرث دولة وعملة السجل الأصلي.</p></section>
      <div className="card-header">
        <div className="card-title"><CircleHelp size={17} strokeWidth={1.8} style={{ verticalAlign: '-3px' }} /> {t('help.title')}</div>
      </div>
      <p style={{ fontSize: 14, lineHeight: 1.7, color: 'var(--muted-foreground)', margin: '8px 0 0' }}>
        {t('help.intro')}
      </p>

      <h2 style={{ fontSize: 16, margin: '18px 0 10px', color: 'var(--primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
        <BookOpen size={16} strokeWidth={1.8} aria-hidden />
        {t('help.sections')}
      </h2>
      <div className="help-guide-grid">
        {SECTION_GUIDES.filter((g) => g.id !== 'help').map((g) => {
          const Icon = GUIDE_ICONS[g.id] || Layers;
          return (
            <div key={g.id} className="content-card help-guide-card" style={{ marginBottom: 0 }}>
              <h3 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <Icon size={16} strokeWidth={1.8} aria-hidden style={{ color: 'var(--gold)' }} />
                {g.title}
              </h3>
              <p className="section-guide-summary">{g.summary}</p>
              {g.blocks && g.blocks.length > 0 ? (
                <div className="section-guide-hint-blocks" style={{ marginBottom: 10 }}>
                  {g.blocks.map((b) => (
                    <div key={b.title} className="section-guide-hint-block">
                      <div className="section-guide-hint-block-title">{b.title}</div>
                      <p className="section-guide-hint-block-body">{b.body}</p>
                    </div>
                  ))}
                </div>
              ) : null}
              <ol className="section-guide-steps">
                {g.steps.map((step) => (
                  <li key={step}>{step}</li>
                ))}
              </ol>
            </div>
          );
        })}
      </div>

      <h2 style={{ fontSize: 16, margin: '22px 0 10px', color: 'var(--primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
        <Lightbulb size={16} strokeWidth={1.8} aria-hidden />
        {t('help.extra')}
      </h2>
      <div className="help-guide-grid">
        {topics.map((section) => (
          <div key={section.id} className="content-card" style={{ marginBottom: 0 }}>
            <h3 style={{ marginBottom: 12, color: 'var(--primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
              <span>{section.icon}</span> {section.title}
            </h3>
            <ul style={{ paddingInlineStart: 20, lineHeight: 2, margin: 0 }}>
              {section.items.map((item, i) => (
                <li key={i} style={{ fontSize: 13 }}>{item}</li>
              ))}
            </ul>
          </div>
        ))}
      </div>

      <div className="content-card help-agreement-card">
        <h2>حول البرنامج والتواصل</h2>
        <div className="license-agreement-text" dir="rtl">
          <p>
            مكتب الوسيط العقاري - OBR هو برنامج مكتبي مخصص للوسيط والمسوق العقاري لتنظيم أعماله وبياناته محليًا على جهازه.
          </p>
          <p style={{ marginBottom: 6 }}>للحصول على نسخة أخرى، أو للاستفسارات والدعم والتواصل مع مطور ومبرمج هذه النسخة:</p>
          <ul style={{ listStyle: 'none', paddingInlineStart: 0, lineHeight: 2 }}>
            <li><strong>الموقع الرسمي:</strong> <a href="https://obr.obr432.workers.dev/" target="_blank" rel="noreferrer" dir="ltr">https://obr.obr432.workers.dev/</a></li>
            <li><strong>البريد الإلكتروني:</strong> <a href="mailto:obr432@gmail.com" dir="ltr">obr432@gmail.com</a></li>
          </ul>
          <p>جميع حقوق التصميم والبرمجة محفوظة لـ OBR.</p>
        </div>
      </div>

      <div className="alert-note help-rights-notice" style={{ marginTop: 20 }}>
        {t('help.rights-notice')}
      </div>
    </div>
  );
}
