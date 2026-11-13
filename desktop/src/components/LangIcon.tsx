/** أيقونة لغة الواجهة: ع فوق A — HTML واضح لأن نص SVG العربي غالبًا لا يُرسم. */
export default function LangIcon({ size = 16 }: { size?: number }) {
  const fs = Math.max(7, Math.round(size * 0.52));
  return (
    <span
      aria-hidden
      className="lang-icon"
      style={{
        width: size,
        height: size,
        display: 'inline-block',
        position: 'relative',
        flexShrink: 0,
        color: 'currentColor',
        fontFamily: 'Tahoma, "Segoe UI", "Arial Unicode MS", sans-serif',
        fontWeight: 800,
        lineHeight: 1,
        fontSize: fs
      }}
    >
      <span
        style={{
          position: 'absolute',
          top: 0,
          insetInlineStart: 0,
          letterSpacing: 0
        }}
      >
        ع
      </span>
      <span
        style={{
          position: 'absolute',
          bottom: 0,
          insetInlineEnd: 0,
          fontFamily: 'Segoe UI, Arial, sans-serif'
        }}
      >
        A
      </span>
    </span>
  );
}
