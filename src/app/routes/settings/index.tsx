import { useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { Moon, Sun, Loader2, MonitorSmartphone, KeyRound } from 'lucide-react';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { commands } from '@/shared/lib/ipc';
import { useWeather } from '@/features/dashboard/hooks';
import { useSettings, usePatchSettings } from '@/features/settings/hooks';
import { useTheme } from '@/app/providers/ThemeProvider';
import { useAgents } from '@/features/bench/hooks';
import McpSyncCard from '@/features/agent/McpSyncCard';
import PluginManageCard from '@/features/plugins/PluginManageCard';

const ACCENTS = ['#ff8a3d', '#f0563b', '#4ea1ff', '#40b881', '#a06bff', '#e8b23c'];

export default function SettingsPage() {
  const { patch, isSaving } = usePatchSettings();
  const { data: settings } = useSettings();
  const { theme, accent } = useTheme();
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [version, setVersion] = useState('');
  const [enteringDesktop, setEnteringDesktop] = useState(false);
  const [desktopActive, setDesktopActive] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    commands.appHealth().then((h) => setVersion(h.version)).catch(() => {});
    isEnabled().then(setAutostart).catch(() => setAutostart(false));
    // 已处于桌面接管态时，本页按钮从「进入」切成「返回桌面主页」（配合左上角返回胶囊）
    commands.desktopModeIsActive().then(setDesktopActive).catch(() => {});
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
          {desktopActive ? (
            <button
              onClick={() => navigate('/desktop')}
              className="flex items-center gap-1.5 rounded-lg bg-[var(--accent)] px-3.5 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90"
            >
              <MonitorSmartphone size={13} />
              返回桌面主页
            </button>
          ) : (
            <button
              onClick={enterDesktop}
              disabled={enteringDesktop}
              className="flex items-center gap-1.5 rounded-lg bg-[var(--accent)] px-3.5 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-60"
            >
              {enteringDesktop ? <Loader2 size={13} className="animate-spin" /> : <MonitorSmartphone size={13} />}
              进入桌面模式
            </button>
          )}
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

        <AssistantEngineCard />

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
        <AiDirectCard />
        <McpSyncCard />
        <PluginManageCard />

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

/** 桌面助手默认引擎：代理 / 模型 / 推理强度（空 = 各自默认；选项来自 agent 画像） */
function AssistantEngineCard() {
  const { patch } = usePatchSettings();
  const { data: settings } = useSettings();
  const agents = useAgents();
  const agentId = settings?.agent.assistant_agent_id ?? '';
  const model = settings?.agent.assistant_model ?? '';
  const effort = settings?.agent.assistant_effort ?? '';

  const selectable = useMemo(
    () => (agents.query.data ?? []).filter((a) => a.installed && a.streaming),
    [agents.query.data],
  );
  const agent = selectable.find((a) => a.id === agentId) ?? null;

  const inputCls =
    'h-8 rounded-lg border border-[var(--border)] bg-transparent px-2.5 text-xs outline-none focus:border-[var(--accent)]';

  return (
    <section className="card px-4 py-3.5">
      <div>
        <div className="text-sm font-medium">桌面助手默认引擎</div>
        <div className="mt-0.5 text-xs text-[var(--text-muted)]">
          桌面助手用哪个 Agent、什么模型与推理强度启动（自动 = claude → zcode → 首个可用）
        </div>
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <select
          value={agentId}
          onChange={(e) => {
            const next = e.target.value;
            const na = selectable.find((a) => a.id === next) ?? null;
            // 换代理时模型/强度随其默认重置，避免残留上一家的取值
            patch({
              agent: {
                assistant_agent_id: next,
                assistant_model: na?.launchOptions?.model?.default ?? '',
                assistant_effort: na?.launchOptions?.effort?.default ?? '',
              },
            });
          }}
          className={`${inputCls} min-w-[140px]`}
          title="桌面助手由哪个本机 Agent 驱动"
        >
          <option value="">自动选择代理</option>
          {selectable.map((a) => (
            <option key={a.id} value={a.id}>
              {a.name}
            </option>
          ))}
        </select>
        {agent ? (
          <>
            {agent.launchOptions?.model && (
              <select
                value={model}
                onChange={(e) => patch({ agent: { assistant_model: e.target.value } })}
                className={inputCls}
                title="默认模型"
              >
                <option value="">默认模型</option>
                {agent.launchOptions.model.choices.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            )}
            {agent.launchOptions?.effort && (
              <select
                value={effort}
                onChange={(e) => patch({ agent: { assistant_effort: e.target.value } })}
                className={inputCls}
                title="默认推理强度"
              >
                <option value="">默认推理强度</option>
                {agent.launchOptions.effort.choices.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            )}
          </>
        ) : (
          <span className="text-xs text-[var(--text-muted)]">模型跟随被选中代理的自身默认</span>
        )}
        {agents.query.isLoading && <Loader2 size={13} className="animate-spin text-[var(--text-muted)]" />}
      </div>
    </section>
  );
}

/** 快问 AI 直连：OpenAI 兼容 base_url / 模型名 / API Key（本地草稿 + 保存） */
function AiDirectCard() {
  const { patch, isSaving } = usePatchSettings();
  const { data: settings } = useSettings();
  const ai = settings?.ai;
  const [draft, setDraft] = useState<{ base_url: string; model: string; api_key: string } | null>(null);
  const [reveal, setReveal] = useState(false);

  const saved = {
    base_url: ai?.base_url ?? '',
    model: ai?.model ?? '',
    api_key: ai?.api_key ?? '',
  };
  const value = draft ?? saved;
  const dirty =
    draft !== null &&
    (draft.base_url !== saved.base_url ||
      draft.model !== saved.model ||
      draft.api_key !== saved.api_key);

  const save = () => {
    if (!draft) return;
    patch({
      ai: {
        base_url: draft.base_url.trim(),
        model: draft.model.trim(),
        api_key: draft.api_key.trim() || null,
      },
    });
    setDraft(null);
  };

  const inputCls =
    'h-8 rounded-lg border border-[var(--border)] bg-transparent px-2.5 text-xs outline-none focus:border-[var(--accent)]';

  return (
    <section className="card px-4 py-3.5">
      <div className="flex items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-1.5 text-sm font-medium">
            <KeyRound size={13} className="text-[var(--text-muted)]" />
            快问 AI 直连
          </div>
          <div className="mt-0.5 text-xs text-[var(--text-muted)]">
            OpenAI 兼容接口（{saved.base_url || '未配置地址'}/chat/completions）；仅用于快问，配置保存在本机
          </div>
        </div>
        {dirty && (
          <button
            onClick={save}
            disabled={isSaving}
            className="shrink-0 rounded-lg bg-[var(--accent)] px-3 py-1.5 text-xs font-medium text-white disabled:opacity-60"
          >
            保存
          </button>
        )}
      </div>
      <div className="mt-3 flex flex-col gap-2">
        <input
          value={value.base_url}
          onChange={(e) => setDraft({ ...value, base_url: e.target.value })}
          placeholder="API 地址，如 https://open.bigmodel.cn/api/paas/v4"
          className={`${inputCls} w-full font-mono`}
        />
        <div className="flex gap-2">
          <input
            value={value.model}
            onChange={(e) => setDraft({ ...value, model: e.target.value })}
            placeholder="模型名，如 glm-4.6"
            className={`${inputCls} flex-1 font-mono`}
          />
          <div className="relative flex-1">
            <input
              value={value.api_key}
              onChange={(e) => setDraft({ ...value, api_key: e.target.value })}
              type={reveal ? 'text' : 'password'}
              placeholder="API Key（留空 = 未配置）"
              className={`${inputCls} w-full pr-8 font-mono`}
            />
            <button
              onClick={() => setReveal((v) => !v)}
              title={reveal ? '隐藏' : '显示'}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-[var(--text-muted)] hover:text-[var(--text)]"
            >
              {reveal ? '隐藏' : '显示'}
            </button>
          </div>
        </div>
      </div>
    </section>
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
