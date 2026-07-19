import { V3_TEXT } from "./tokens.js";

export function ControlField({
  label,
  min,
  max,
  onChange,
  placeholder,
  type = "text",
  value,
}) {
  return (
    <label className="block">
      <span className={`text-[10px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</span>
      <div className="surface-inset mt-2 flex h-11 items-center rounded-xl px-3">
        <input
          className={`w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong}`}
          max={max}
          min={min ?? (type === "number" ? 1 : undefined)}
          onChange={(event) => onChange(event.target.value)}
          placeholder={placeholder}
          type={type}
          value={value}
        />
      </div>
    </label>
  );
}

export function ControlSelectField({ label, onChange, options, value }) {
  return (
    <label className="block">
      <span className={`text-[10px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</span>
      <div className="surface-inset mt-2 flex h-11 items-center rounded-xl px-3">
        <select
          className={`w-full bg-transparent text-[12px] outline-none ${V3_TEXT.strong}`}
          onChange={(event) => onChange(event.target.value)}
          value={value}
        >
          {options.map((option) => (
            <option key={option.value} value={option.value}>{option.label}</option>
          ))}
        </select>
      </div>
    </label>
  );
}

export function ControlButton({
  children,
  disabled,
  onClick,
  primary = false,
  tone = "normal",
}) {
  const toneClass = tone === "danger" ? "text-red-800 dark:text-red-300" : V3_TEXT.body;
  return (
    <button
      className={primary
        ? "h-10 rounded-full px-4 text-[12px] font-semibold text-white disabled:opacity-40"
        : `surface-inset h-10 rounded-full px-4 text-[12px] disabled:opacity-40 ${toneClass}`}
      disabled={disabled}
      onClick={onClick}
      style={primary ? { backgroundImage: "linear-gradient(90deg, var(--accent), var(--accent-2))" } : undefined}
      type="button"
    >
      {children}
    </button>
  );
}

export function ControlPanelTitle({ children, icon: Icon, subtitle }) {
  return (
    <div>
      <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>
        {Icon ? <Icon size={12} /> : null}
        {children}
      </div>
      {subtitle ? <p className={`mt-2 text-[12px] leading-relaxed ${V3_TEXT.mute}`}>{subtitle}</p> : null}
    </div>
  );
}

export function ProductControlsLayout({
  children,
  description,
  eyebrow = "Run operations",
  message,
  title = "Presets, schedules, and scope.",
}) {
  return (
    <div className="space-y-6">
      <section className="surface p-6 sm:p-8">
        <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>{eyebrow}</div>
        <h1 className={`mt-2 font-display text-[42px] font-semibold ${V3_TEXT.strong}`}>{title}</h1>
        <p className={`mt-3 max-w-3xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{description}</p>
        {message ? <div className={`surface-inset mt-5 rounded-xl p-3 text-[12px] ${V3_TEXT.body}`}>{message}</div> : null}
      </section>
      {children}
    </div>
  );
}

export function ProductControlsPair({ children }) {
  return <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">{children}</div>;
}

export function ProductControlSection({ children, className = "" }) {
  return <section className={`surface p-6 ${className}`.trim()}>{children}</section>;
}

export function ProductControlsSafetyBoundary({ cards, subtitle }) {
  return (
    <ProductControlSection>
      <ControlPanelTitle subtitle={subtitle}>Safety boundary</ControlPanelTitle>
      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        {cards.map((card) => (
          <div className={`surface-inset rounded-xl p-4 text-[11px] leading-relaxed ${V3_TEXT.mute}`} key={card.title}>
            <span className={`block font-display text-[15px] ${V3_TEXT.strong}`}>{card.title}</span>
            {card.body}
          </div>
        ))}
      </div>
    </ProductControlSection>
  );
}
