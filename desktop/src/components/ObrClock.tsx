import React, { useEffect, useId, useState } from 'react';
import BrandLogo from './BrandLogo';

function hands(now: Date) {
  const seconds = now.getSeconds() + now.getMilliseconds() / 1000;
  const minutes = now.getMinutes() + seconds / 60;
  const hours = (now.getHours() % 12) + minutes / 60;
  return {
    hour: hours * 30,
    minute: minutes * 6,
    second: seconds * 6
  };
}

export default function ObrClock({ showBrand = true }: { showBrand?: boolean }) {
  const [now, setNow] = useState(() => new Date());
  const uid = useId().replace(/:/g, '');

  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 250);
    return () => window.clearInterval(id);
  }, []);

  const { hour, minute, second } = hands(now);
  const digital = now.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit', second: '2-digit' });

  return (
    <div className="obr-clock" role="img" aria-label={`الساعة الآن ${digital}`}>
      <svg viewBox="0 0 280 280" className="obr-clock-face">
        <defs>
          <filter id={`obr-clock-shadow-${uid}`} x="-25%" y="-25%" width="150%" height="150%">
            <feDropShadow dx="0" dy="8" stdDeviation="10" floodColor="var(--primary-dark)" floodOpacity="0.16" />
            <feDropShadow dx="0" dy="2" stdDeviation="2.5" floodColor="var(--primary-dark)" floodOpacity="0.08" />
          </filter>
          <filter id={`obr-hand-shadow-${uid}`} x="-40%" y="-40%" width="180%" height="180%">
            <feDropShadow dx="0" dy="1.4" stdDeviation="1.2" floodColor="var(--primary-dark)" floodOpacity="0.22" />
          </filter>
          <radialGradient id={`obr-bezel-${uid}`} cx="38%" cy="30%" r="75%">
            <stop offset="0%" stopColor="var(--card)" />
            <stop offset="62%" stopColor="var(--surface)" />
            <stop offset="100%" stopColor="var(--border)" />
          </radialGradient>
          <radialGradient id={`obr-face-${uid}`} cx="42%" cy="32%" r="72%">
            <stop offset="0%" stopColor="var(--card)" />
            <stop offset="100%" stopColor="var(--surface)" />
          </radialGradient>
          <linearGradient id={`obr-wave-${uid}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--primary)" />
            <stop offset="100%" stopColor="var(--primary-dark)" />
          </linearGradient>
          <clipPath id={`obr-wave-clip-${uid}`}>
            <circle cx="140" cy="140" r="86" />
          </clipPath>
        </defs>

        <circle cx="140" cy="140" r="108" fill={`url(#obr-bezel-${uid})`} filter={`url(#obr-clock-shadow-${uid})`} />
        <circle cx="140" cy="140" r="108" fill="none" stroke="var(--gold)" strokeWidth="1.15" strokeOpacity="0.7" />
        <circle cx="140" cy="140" r="88" fill={`url(#obr-face-${uid})`} />
        <circle cx="140" cy="140" r="88" fill="none" stroke="var(--primary-dark)" strokeWidth="0.6" strokeOpacity="0.08" />
        <path
          d="M54 168 C96 152 150 198 226 162 L226 226 L54 226 Z"
          fill={`url(#obr-wave-${uid})`}
          clipPath={`url(#obr-wave-clip-${uid})`}
          opacity="0.94"
        />

        {Array.from({ length: 12 }, (_, i) => {
          const deg = i * 30;
          const major = i % 3 === 0;
          return (
            <rect
              key={deg}
              x={major ? 137.2 : 138.2}
              y="58"
              width={major ? 5.6 : 3.6}
              height={major ? 12 : 8}
              rx="2"
              fill={major ? 'var(--primary-dark)' : 'var(--muted-foreground)'}
              opacity={major ? 0.82 : 0.45}
              transform={`rotate(${deg} 140 140)`}
            />
          );
        })}

        <text x="140" y="82" textAnchor="middle" className="obr-clock-num">12</text>
        <text x="198" y="146" textAnchor="middle" className="obr-clock-num">3</text>
        <text x="140" y="214" textAnchor="middle" className="obr-clock-num obr-clock-num-light">6</text>
        <text x="82" y="146" textAnchor="middle" className="obr-clock-num">9</text>

        <g className="obr-clock-hand obr-clock-hand-hour" transform={`rotate(${hour} 140 140)`} filter={`url(#obr-hand-shadow-${uid})`}>
          <path
            d="M133.2 154 L134.4 102 L140 82 L145.6 102 L146.8 154 L143.2 160 L136.8 160 Z"
            fill="var(--card)"
            stroke="var(--primary-dark)"
            strokeWidth="1.8"
            strokeLinejoin="round"
          />
          <path d="M140 154 L140 90" stroke="var(--gold)" strokeWidth="2.2" strokeLinecap="round" />
        </g>
        <g className="obr-clock-hand obr-clock-hand-minute" transform={`rotate(${minute} 140 140)`} filter={`url(#obr-hand-shadow-${uid})`}>
          <path
            d="M136.4 156 L137.4 80 L140 58 L142.6 80 L143.6 156 L141.4 162 L138.6 162 Z"
            fill="var(--card)"
            stroke="var(--primary-dark)"
            strokeWidth="1.6"
            strokeLinejoin="round"
          />
        </g>
        <g transform={`rotate(${second} 140 140)`}>
          <line x1="140" y1="158" x2="140" y2="62" stroke="var(--gold-bright)" strokeWidth="2.2" strokeLinecap="round" />
        </g>
        <circle cx="140" cy="140" r="8.2" fill="var(--primary-dark)" />
        <circle cx="140" cy="140" r="7.2" fill="var(--gold-bright)" filter={`url(#obr-hand-shadow-${uid})`} />
        <circle cx="140" cy="140" r="3.1" fill="var(--card)" />
      </svg>
      {showBrand && (
        <div className="obr-clock-brand">
          <BrandLogo className="obr-clock-logo" />
          <div className="obr-clock-digital num" aria-hidden="true">{digital}</div>
        </div>
      )}
      {!showBrand && (
        <div className="obr-clock-digital num" aria-hidden="true">{digital}</div>
      )}
    </div>
  );
}
