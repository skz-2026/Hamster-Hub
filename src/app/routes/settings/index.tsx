import { useEffect, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Moon, Sun, Loader2, MonitorSmartphone } from 'lucide-react';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { commands } from '@/shared/lib/ipc';
import { useWeather } from '@/features/dashboard/hooks';
import { useSettings, usePatchSettings } from '@/features/settings/hooks';
import { useTheme } from '@/app/providers/ThemeProvider';
import McpSyncCard from '@/features/agent/McpSyncCard';

const ACCENTS = ['#ff8a3d', '#f0563b', '#4ea1ff', '#40b881', '#a06bff', '#e8b23c'];

export default function SettingsPage() {
  const { patch, isSaving } = usePatchSettings();
  const { data: settings } = useSettings();
  const { theme, accent } = useTheme();
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [version, setVersion] = useState('');
  const [enteringDesktop, setEnteringDesktop] = useState(false);

  useEffect(() => {
    commands.appHealth().then((h) => setVersion(h.version)).catch(() => {});
    isEnabled().then(setAutostart).catch(() => setAutostart(false));
  }, []);

  const toggleAutostart = async () => {
    const next = !(autostart ?? false);
    try {
      await (next ? enable() : disable());
      setAutostart(next);
    } catch {
      /* 权限或系统限制，保持原状 */
    }
  };

  const enterDesktop = async () => {
    setEnteringDesktop(true);
    try {
      await commands.desktopModeEnter();
    } catch (e) {
      console.error('[settings] 进入桌面模式失败', e);
    } finally {
      setEnteringDesktop(false);
    }
  };

  return (
    <div className="mx-auto max-w-2xl">
      <div className="mb-5 flex items-center justify-between">
        <h1 className="text-xl font-semibold">设置</h1>
        {isSaving && <Loader2 size={16} className="animate-spin text-[var(--text-muted)]" />}
      </div>

      <div className="flex flex-col gap-3">
        <Row title="桌面模式" desc="全屏工作台 + 底部 Dock（任务栏保留可见，打开的应用永不遮挡）">
          <button
            onClick={enterDesktop}
            disabled={enteringDesktop}
            className="flex items-center gap-1.5 rounded-lg bg-[var(--accent)] px-3.5 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-60"
          >
            {enteringDesktop ? <Loader2 size={13} className="animate-spin" /> : <MonitorSmartphone size={13} />}
            进入桌面模式
          </button>
        </Row>

        <Row title="启动进入桌面模式" desc="打开仓鼠Hub 直接全屏进入（对齐水豚hub，重启后生效）">
          <Switch
            checked={settings?.behavior.desktop_mode_on_launch ?? true}
            onChange={(v) => patch({ behavior: { desktop_mode_on_launch: v } })}
          />
        </Row>


        <Row title="主题" desc="深色毛玻璃 / 浅色奶油">
          <div className="flex gap-2">
            <ThemeButton active={theme === 'dark'} onClick={() => patch({ appearance: { theme: 'dark' } })}>
              <Moon size={15} /> 深色
            </ThemeButton>
            <ThemeButton active={theme === 'light'} onClick={() => patch({ appearance: { theme: 'light' } })}>
              <Sun size={15} /> 浅色
            </ThemeButton>
          </div>
        </Row>

        <Row title="强调色" desc="按钮与高亮的主色调">
          <div className="flex items-center gap-2">
            {ACCENTS.map((c) => (
              <button
                key={c}
                title={c}
                onClick={() => patch({ appearance: { accent: c } })}
                className={`size-6 rounded-full transition-transform hover:scale-110 ${
                  accent.toLowerCase() === c ? 'ring-2 ring-white/70 ring-offset-2 ring-offset-[var(--bg)]' : ''
                }`}
                style={{ background: c }}
              />
            ))}
          </div>
        </Row>

        <Row title="天气城市" desc="工作台天气卡显示的城市（Open-Meteo 数据）">
          <CityInput />
        </Row>

        <HotkeyRow
          title="Spotlight 热键"
          desc="呼出全局搜索（重启后生效；被占用时可用托盘菜单）"
          value={settings?.search.hotkey ?? ''}
          onSave={(v) => patch({ search: { hotkey: v } })}
        />
        <HotkeyRow
          title="桌面模式热键"
          desc="进入/退出 iOS 主屏（重启后生效；也可用托盘菜单）"
          value={settings?.behavior.desktop_mode_hotkey ?? ''}
          onSave={(v) => patch({ behavior: { desktop_mode_hotkey: v } })}
        />

        <Row
          title="允许操作电脑（Computer Use）"
          desc="桌面助手可经桌面工具截图并操作真实鼠标键盘（默认关；开启后每次调用留审计，急停 = 结束会话）"
        >
          <Switch
            checked={settings?.agent.computer_use_enabled ?? false}
            onChange={(v) => patch({ agent: { computer_use_enabled: v } })}
          />
        </Row>
        <PersonaRow
          value={settings?.agent.assistant_persona ?? ''}
          onSave={(v) => patch({ agent: { assistant_persona: v } })}
        />
        <McpSyncCard />

        <Row title="开机自启" desc="随 Windows 启动并最小化到托盘">
          <Switch checked={autostart ?? false} onChange={() => toggleAutostart()} />
        </Row>

        <Row title="关于" desc={`仓鼠Hub ${version ? `v${version}` : ''} · 本地优先，数据不出本机`}>
          <a
            className="text-xs text-[var(--text-muted)] underline-offset-2 hover:underline"
            href="https://github.com/"
            target="_blank"
            rel="noreferrer"
          >
            反馈与更新
          </a>
        </Row>
      </div>
    </div>
  );
}

function Row({ title, desc, children }: { title: string; desc: string; children: React.ReactNode }) {
  return (
    <section className="card flex items-center justify-between px-4 py-3.5">
      <div>
        <div className="text-sm font-medium">{title}</div>
        <div className="mt-0.5 text-xs text-[var(--text-muted)]">{desc}</div>
      </div>
      {children}
    </section>
  );
}

/** 桌面助手人设（本地草稿 + 保存；空 = Rust 侧内置仓鼠默认 persona） */
function PersonaRow({ value, onSave }: { value: string; onSave: (v: string) => void }) {
  const [draft, setDraft] = useState<string | null>(null);
  const dirty = draft !== null && draft !== value;
  return (
    <Row title="桌面助手人设" desc="追加为 Agent 的系统设定（空 = 内置仓鼠默认）">
      <div className="flex items-start gap-2">
        <textarea
          value={draft ?? value}
          onChange={(e) => setDraft(e.target.value)}
          rows={3}
          placeholder="例如：回答尽量精简，先给结论。"
          className="w-56 resize-none rounded-lg border border-[var(--border)] bg-transparent px-2.5 py-1.5 text-xs outline-none focus:border-[var(--accent)]"
        />
        {dirty && (
          <button
            onClick={() => onSave(draft!)}
            className="rounded-lg bg-[var(--accent)] px-3 py-1.5 text-xs font-medium text-white"
          >
            保存
          </button>
        )}
      </div>
    </Row>
  );
}

/** 热键编辑：本地草稿 + 保存（重启后由 Rust 重新注册生效） */
function HotkeyRow({
  title,
  desc,
  value,
  onSave,
}: {
  title: string;
  desc: string;
  value: string;
  onSave: (v: string) => void;
}) {
  const [draft, setDraft] = useState<string | null>(null);
  const dirty = draft !== null && draft.trim() !== '' && draft.trim() !== value;
  return (
    <Row title={title} desc={desc}>
      <div className="flex items-center gap-2">
        <input
          value={draft ?? value}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && dirty) onSave(draft!.trim());
            if (e.key === 'Escape') setDraft(null);
          }}
          placeholder="如 Ctrl+Shift+S"
          className="h-8 w-40 rounded-lg border border-[var(--border)] bg-transparent px-2.5 font-mono text-xs outline-none focus:border-[var(--accent)]"
        />
        {dirty && (
          <button
            onClick={() => onSave(draft!.trim())}
            className="rounded-lg bg-[var(--accent)] px-3 py-1.5 text-xs font-medium text-white"
          >
            保存
          </button>
        )}
      </div>
    </Row>
  );
}

/** 天气城市输入（本地草稿 + 应用内生效） */
function CityInput() {  const queryClient = useQueryClient();
  const { data: w } = useWeather();
  const [city, setCity] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const value = city ?? w?.city ?? '';
  const dirty = city !== null && city !== (w?.city ?? '');

  const apply = async () => {
    if (!dirty || !city.trim()) return;
    setSaving(true);
    try {
      await commands.weatherSetCity(city.trim());
      await queryClient.invalidateQueries({ queryKey: ['weather'] });
      setCity(null);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex items-center gap-2">
      <input
        value={value}
        onChange={(e) => setCity(e.target.value)}
        onKeyDown={(e) => e.key === 'Enter' && apply()}
        placeholder="城市名，如 北京"
        className="h-8 w-36 rounded-lg border border-[var(--border)] bg-transparent px-2.5 text-xs outline-none focus:border-[var(--accent)]"
      />
      {dirty && (
        <button
          onClick={apply}
          disabled={saving}
          className="rounded-lg bg-[var(--accent)] px-3 py-1.5 text-xs font-medium text-white disabled:opacity-50"
        >
          {saving ? <Loader2 size={12} className="animate-spin" /> : '应用'}
        </button>
      )}
    </div>
  );
}

function ThemeButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 rounded-lg border px-3 py-1.5 text-xs transition-colors ${
        active
          ? 'border-[var(--accent)] bg-[var(--accent-weak)] text-[var(--accent)]'
          : 'border-[var(--border)] text-[var(--text-muted)] hover:text-[var(--text)]'
      }`}
    >
      {children}
    </button>
  );
}

function Switch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-11 rounded-full transition-colors ${
        checked ? 'bg-[var(--accent)]' : 'bg-[var(--panel-strong)] border border-[var(--border)]'
      }`}
    >
      <span
        className={`absolute top-0.5 size-5 rounded-full bg-white shadow transition-all ${
          checked ? 'left-[22px]' : 'left-0.5'
        }`}
      />
    </button>
  );
}
