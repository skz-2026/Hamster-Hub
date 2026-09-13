/**
 * 设置页：分组卡（外观 / 桌面模式 / AI 与助手 / 插件 / 系统）+ 吸顶锚点导航。
 * 交互约定：开关与选择类即点即存；文本类统一 AutoSaveField（失焦 / Enter 保存，
 * Esc 还原，行内状态反馈），页内不再出现「保存 / 应用」按钮。
 */
import { useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { ChevronDown, ChevronRight, Loader2, MonitorSmartphone, KeyRound, Moon, Sun, Gamepad2, Bot, FolderCog } from 'lucide-react';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { commands } from '@/shared/lib/ipc';
import { useWeather } from '@/features/dashboard/hooks';
import { useSettings, usePatchSettings } from '@/features/settings/hooks';
import {
  Section,
  Group,
  Row,
  BlockRow,
  Switch,
  ThemeButton,
  AutoSaveField,
} from '@/features/settings/controls';
import { useTheme } from '@/app/providers/ThemeProvider';
import { useI18n } from '@/shared/i18n/provider';
import type { AgentInfo } from '@/shared/types/bench';
import { useAgents } from '@/features/bench/hooks';
import SelectPill from '@/features/bench/SelectPill';
import AgentAvatar from '@/features/bench/AgentAvatar';
import McpSyncCard from '@/features/agent/McpSyncCard';
import PluginManageCard from '@/features/plugins/PluginManageCard';
import { WallpaperPicker } from '@/features/settings/WallpaperPicker';
import UpdaterRow from '@/features/settings/UpdaterRow';

const FEEDBACK_URL = 'https://github.com/skz-2026/Hamster-Hub/issues';

const ACCENTS = ['#ff8a3d', '#f0563b', '#4ea1ff', '#40b881', '#a06bff', '#e8b23c'];

/** label 存字面量 i18n key（显式映射，禁止动态拼 key），渲染时经 t() 翻译 */
const SECTIONS = [
  { id: 'appearance', label: 'settings.section.appearance' },
  { id: 'desktop', label: 'settings.section.desktop' },
  { id: 'assistant', label: 'settings.section.assistant' },
  { id: 'plugins', label: 'settings.section.plugins' },
  { id: 'system', label: 'settings.section.system' },
] as const;

export default function SettingsPage() {
  const { patch, patchAsync, isSaving } = usePatchSettings();
  const { data: settings } = useSettings();
  const { theme, accent } = useTheme();
  const { lang, langs, setLang, t } = useI18n();
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
      {/* 吸顶页头 + 锚点导航（滚动到任意位置都能直达目标分组） */}
      <div className="sticky top-0 z-30 bg-[var(--bg)] pb-3 pt-1">
        <div className="mb-2.5 flex items-center justify-between">
          <h1 className="text-xl font-semibold">{t('settings.title')}</h1>
          {isSaving && <Loader2 size={16} className="animate-spin text-[var(--text-muted)]" />}
        </div>
        <nav className="flex flex-wrap gap-1.5">
          {SECTIONS.map((s) => (
            <button
              key={s.id}
              onClick={() => document.getElementById(s.id)?.scrollIntoView({ block: 'start' })}
              className="rounded-full border border-[var(--border)] px-3 py-1 text-[11.5px] text-[var(--text-muted)] transition-colors hover:border-[var(--accent)] hover:text-[var(--text)]"
            >
              {t(s.label)}
            </button>
          ))}
        </nav>
      </div>

      <div className="flex flex-col gap-6">
        {/* ===== 外观 ===== */}
        <Section id="appearance" title={t('settings.section.appearance')}>
          <Group>
            <Row title={t('settings.theme.label')} desc={t('settings.theme.desc')}>
              <div className="flex gap-2">
                <ThemeButton active={theme === 'dark'} onClick={() => patch({ appearance: { theme: 'dark' } })}>
                  <Moon size={15} /> {t('settings.theme.dark')}
                </ThemeButton>
                <ThemeButton active={theme === 'light'} onClick={() => patch({ appearance: { theme: 'light' } })}>
                  <Sun size={15} /> {t('settings.theme.light')}
                </ThemeButton>
                <ThemeButton active={theme === 'pixel'} onClick={() => patch({ appearance: { theme: 'pixel' } })}>
                  <Gamepad2 size={15} /> {t('settings.theme.pixel')}
                </ThemeButton>
              </div>
            </Row>
            <Row title={t('settings.language.label')} desc={t('settings.language.desc')}>
              <div className="flex gap-2">
                {langs.map((l) => (
                  <ThemeButton key={l.id} active={lang === l.id} onClick={() => setLang(l.id)}>
                    {l.label}
                  </ThemeButton>
                ))}
              </div>
            </Row>
            <Row title={t('settings.accent.label')} desc={t('settings.accent.desc')}>
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
            <BlockRow
              title={t('settings.wallpaper.label')}
              desc={t('settings.wallpaper.desc')}
            >
              <WallpaperPicker />
            </BlockRow>
          </Group>
        </Section>

        {/* ===== 桌面模式 ===== */}
        <Section id="desktop" title={t('settings.section.desktop')}>
          <Group>
            <Row title={t('settings.desktopMode.label')} desc={t('settings.desktopMode.desc')}>
              {desktopActive ? (
                <button
                  onClick={() => navigate('/')}
                  className="flex shrink-0 items-center gap-1.5 rounded-lg bg-[var(--accent)] px-3.5 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90"
                >
                  <MonitorSmartphone size={13} />
                  {t('settings.desktopMode.backToHome')}
                </button>
              ) : (
                <button
                  onClick={enterDesktop}
                  disabled={enteringDesktop}
                  className="flex shrink-0 items-center gap-1.5 rounded-lg bg-[var(--accent)] px-3.5 py-1.5 text-xs font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-60"
                >
                  {enteringDesktop ? <Loader2 size={13} className="animate-spin" /> : <MonitorSmartphone size={13} />}
                  {t('settings.desktopMode.enter')}
                </button>
              )}
            </Row>
            <Row title={t('settings.desktopMode.launch.label')} desc={t('settings.desktopMode.launch.desc')}>
              <Switch
                checked={settings?.behavior.desktop_mode_on_launch ?? true}
                onChange={(v) => patch({ behavior: { desktop_mode_on_launch: v } })}
              />
            </Row>
            <Row title={t('settings.desktopMode.hotkey.label')} desc={t('settings.desktopMode.hotkey.desc')}>
              <AutoSaveField
                value={settings?.behavior.desktop_mode_hotkey ?? ''}
                allowEmpty={false}
                mono
                ariaLabel={t('settings.desktopMode.hotkey.label')}
                placeholder={t('settings.desktopMode.hotkey.placeholder')}
                onSave={(v) => patchAsync({ behavior: { desktop_mode_hotkey: v } })}
              />
            </Row>
          </Group>
        </Section>

        {/* ===== AI 与助手 ===== */}
        <AssistantSection />

        {/* ===== 插件 ===== */}
        <Section id="plugins" title={t('settings.section.plugins')}>
          <PluginManageCard />
        </Section>

        {/* ===== 系统 ===== */}
        <Section id="system" title={t('settings.section.system')}>
          <Group>
            <Row title={t('settings.spotlight.label')} desc={t('settings.spotlight.desc')}>
              <AutoSaveField
                value={settings?.search.hotkey ?? ''}
                allowEmpty={false}
                mono
                ariaLabel={t('settings.spotlight.label')}
                placeholder={t('settings.spotlight.placeholder')}
                onSave={(v) => patch({ search: { hotkey: v } })}
              />
            </Row>
            <Row title={t('settings.weather.label')} desc={t('settings.weather.desc')}>
              <CityField />
            </Row>
            <Row title={t('settings.autostart.label')} desc={t('settings.autostart.desc')}>
              <Switch checked={autostart ?? false} onChange={() => toggleAutostart()} />
            </Row>
            <UpdaterRow />
            <Row title={t('settings.about.label')} desc={t('settings.about.desc', { version: version ? `v${version}` : '' })}>
              <button
                className="text-xs text-[var(--text-muted)] underline-offset-2 hover:underline"
                onClick={() => void commands.openUrl(FEEDBACK_URL).catch(() => {})}
              >
                {t('settings.about.feedback')}
              </button>
            </Row>
          </Group>
        </Section>
      </div>
    </div>
  );
}

/** AI 与助手分组：默认引擎（下拉需抬层）+ Computer Use + 人设 + 快问直连 + MCP 分发 */
function AssistantSection() {
  const { patch } = usePatchSettings();
  const { data: settings } = useSettings();
  const { t } = useI18n();
  // 任一下拉展开时抬层，弹层盖过后续分组卡（.card backdrop-filter 层叠上下文）
  const [pillOpen, setPillOpen] = useState(false);

  return (
    <Section id="assistant" title={t('settings.section.assistant')}>
      <Group elevated={pillOpen}>
        <EngineBlock onOpenChange={setPillOpen} />
        <Row
          title={t('settings.computerUse.label')}
          desc={t('settings.computerUse.desc')}
        >
          <Switch
            checked={settings?.agent.computer_use_enabled ?? false}
            onChange={(v) => patch({ agent: { computer_use_enabled: v } })}
          />
        </Row>
        <Row title={t('settings.persona.label')} desc={t('settings.persona.desc')}>
          <PersonaField />
        </Row>
        <AiDirectBlock />
      </Group>
      <Group>
        <CliPathsBlock />
      </Group>
      <McpSyncCard />
    </Section>
  );
}

/** 桌面助手默认引擎：代理 / 模型 / 推理强度（空 = 各自默认；选项来自 agent 画像） */
function EngineBlock({ onOpenChange }: { onOpenChange: (open: boolean) => void }) {
  const { patch } = usePatchSettings();
  const { data: settings } = useSettings();
  const { t } = useI18n();
  const agents = useAgents();
  const agentId = settings?.agent.assistant_agent_id ?? '';
  const model = settings?.agent.assistant_model ?? '';
  const effort = settings?.agent.assistant_effort ?? '';

  const selectable = useMemo(
    () => (agents.query.data ?? []).filter((a) => a.installed && a.streaming),
    [agents.query.data],
  );
  const agent = selectable.find((a) => a.id === agentId) ?? null;

  return (
    <BlockRow
      title={t('settings.engine.label')}
      desc={t('settings.engine.desc')}
    >
      <div className="flex flex-wrap items-center gap-2">
        <SelectPill
          variant="field"
          direction="down"
          onOpenChange={onOpenChange}
          title={t('settings.engine.agentTitle')}
          icon={
            agentId ? (
              <AgentAvatar agentId={agentId} size={14} />
            ) : (
              <Bot size={13} className="text-[var(--text-muted)]" />
            )
          }
          value={agentId}
          options={[
            {
              value: '',
              label: t('settings.engine.autoAgent'),
              hint: t('settings.engine.autoAgentHint'),
              icon: <Bot size={14} className="text-white/50" />,
            },
            ...selectable.map((a) => ({
              value: a.id,
              label: a.name,
              hint: a.version ? `v${a.version}` : undefined,
              icon: <AgentAvatar agentId={a.id} size={16} />,
            })),
          ]}
          onChange={(v) => {
            const na = selectable.find((a) => a.id === v) ?? null;
            // 换代理时模型/强度随其默认重置，避免残留上一家的取值
            patch({
              agent: {
                assistant_agent_id: v,
                assistant_model: na?.launchOptions?.model?.default ?? '',
                assistant_effort: na?.launchOptions?.effort?.default ?? '',
              },
            });
          }}
          placeholder={t('settings.engine.autoAgent')}
        />
        {agent ? (
          <>
            {agent.launchOptions?.model && (
              <SelectPill
                variant="field"
                direction="down"
                onOpenChange={onOpenChange}
                title={t('settings.engine.defaultModel')}
                value={model}
                options={[
                  { value: '', label: t('settings.engine.defaultModel') },
                  ...agent.launchOptions.model.choices.map((c) => ({ value: c, label: c })),
                ]}
                onChange={(v) => patch({ agent: { assistant_model: v } })}
                placeholder={t('settings.engine.defaultModel')}
              />
            )}
            {agent.launchOptions?.effort && (
              <SelectPill
                variant="field"
                direction="down"
                onOpenChange={onOpenChange}
                title={t('settings.engine.defaultEffort')}
                value={effort}
                options={[
                  { value: '', label: t('settings.engine.defaultEffort') },
                  ...agent.launchOptions.effort.choices.map((c) => ({ value: c, label: c })),
                ]}
                onChange={(v) => patch({ agent: { assistant_effort: v } })}
                placeholder={t('settings.engine.defaultEffort')}
              />
            )}
          </>
        ) : (
          <span className="text-xs text-[var(--text-muted)]">{t('settings.engine.followAgentDefault')}</span>
        )}
        {agents.query.isLoading && <Loader2 size={13} className="animate-spin text-[var(--text-muted)]" />}
      </div>
    </BlockRow>
  );
}

function PersonaField() {
  const { patchAsync } = usePatchSettings();
  const { data: settings } = useSettings();
  const { t } = useI18n();
  return (
    <AutoSaveField
      value={settings?.agent.assistant_persona ?? ''}
      multiline
      rows={3}
      className="w-64"
      ariaLabel={t('settings.persona.label')}
      placeholder={t('settings.persona.placeholder')}
      onSave={(v) => patchAsync({ agent: { assistant_persona: v } })}
    />
  );
}

/**
 * Agent CLI 位置：展示每个 Agent 实际将启动的程序路径（自动探测 = 多版本择优），
 * 支持手动指定（绿色版/自拷贝的 CLI 未上 PATH 时注册进应用）与恢复自动。
 * 保存经 agentCliPathSet 先校验（agent id 存在 + 路径是存在的文件）再落库。
 * 只平铺已安装的（按常用度排序）；未安装的默认折叠进「未安装（N）」。
 */

/** 常用度排序（越靠前越常用；未列出的排后面按名称序） */
const CLI_POPULARITY = [
  'claude',
  'codex',
  'gemini',
  'cursor',
  'opencode',
  'kimi',
  'copilot',
  'qwen',
  'zcode',
  'codebuddy',
  'glm',
  'cline',
  'goose',
  'kilo',
  'droid',
  'devin',
  'auggie',
];
const popularityRank = (id: string): number => {
  const i = CLI_POPULARITY.indexOf(id);
  return i === -1 ? CLI_POPULARITY.length : i;
};

function CliPathsBlock() {
  const queryClient = useQueryClient();
  const agents = useAgents();
  const { data: settings } = useSettings();
  const { t } = useI18n();
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState('');
  const [error, setError] = useState('');
  const [busyId, setBusyId] = useState<string | null>(null);
  const [showUninstalled, setShowUninstalled] = useState(false);

  const overrides = settings?.agent.cli_paths ?? {};
  const invalidate = () => {
    void queryClient.invalidateQueries({ queryKey: ['settings'] });
    // program/installed 来自扫描，改路径后刷新
    void queryClient.invalidateQueries({ queryKey: ['bench', 'agents'] });
  };

  const save = async (id: string, raw: string) => {
    setBusyId(id);
    setError('');
    try {
      await commands.agentCliPathSet(id, raw.trim() || null);
      setEditing(null);
      invalidate();
    } catch (e) {
      setError(String(e).replace(/^Error:\s*/, ''));
    } finally {
      setBusyId(null);
    }
  };

  const byPopularity = (a: AgentInfo, b: AgentInfo) =>
    popularityRank(a.id) - popularityRank(b.id) || a.name.localeCompare(b.name);
  const all = [...(agents.query.data ?? [])];
  const installed = all.filter((a) => a.installed).sort(byPopularity);
  const uninstalled = all.filter((a) => !a.installed).sort(byPopularity);

  /** 单行：路径展示 / 编辑态（保存经后端校验，失败行内提示） */
  const renderRow = (a: AgentInfo) => {
    const overridden = overrides[a.id];
    const shown = overridden ?? a.program ?? null;
    const editingThis = editing === a.id;
    return (
      <div key={a.id} className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <AgentAvatar agentId={a.id} size={16} title={a.name} />
          <span className="w-24 shrink-0 truncate text-xs text-[var(--text)]">{a.name}</span>
          {editingThis ? (
            <input
              autoFocus
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.nativeEvent.isComposing) void save(a.id, draft);
                if (e.key === 'Escape') setEditing(null);
              }}
              placeholder={t('settings.cli.pathPlaceholder')}
              className="min-w-0 flex-1 rounded-lg bg-black/25 px-2.5 py-1.5 font-mono text-[11.5px] text-[var(--text)] outline-none ring-1 ring-[var(--border)] focus:ring-[var(--accent)]"
            />
          ) : (
            <span
              className="min-w-0 flex-1 truncate font-mono text-[11px] text-[var(--text-muted)]"
              title={shown ?? t('settings.cli.notDetected')}
            >
              {shown ?? t('settings.cli.notDetected')}
              {overridden && <span className="ml-1.5 text-[10px] text-[var(--accent)]">{t('settings.cli.manualBadge')}</span>}
            </span>
          )}
          {editingThis ? (
            <>
              <button
                onClick={() => void save(a.id, draft)}
                disabled={busyId === a.id}
                className="shrink-0 rounded-lg bg-[var(--accent)] px-2.5 py-1 text-[11px] font-medium text-white transition-opacity hover:opacity-90 disabled:opacity-50"
              >
                {busyId === a.id ? <Loader2 size={11} className="animate-spin" /> : t('settings.action.save')}
              </button>
              <button
                onClick={() => {
                  setEditing(null);
                  setError('');
                }}
                className="shrink-0 rounded-lg px-2 py-1 text-[11px] text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
              >
                {t('settings.action.cancel')}
              </button>
            </>
          ) : (
            <>
              <button
                onClick={() => {
                  setEditing(a.id);
                  setDraft(overridden ?? '');
                  setError('');
                }}
                className="shrink-0 rounded-lg px-2 py-1 text-[11px] text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
              >
                {overridden ? t('settings.cli.edit') : t('settings.cli.specifyPath')}
              </button>
              {overridden && (
                <button
                  onClick={() => void save(a.id, '')}
                  className="shrink-0 rounded-lg px-2 py-1 text-[11px] text-[var(--text-muted)] transition-colors hover:bg-[var(--hover)] hover:text-[var(--text)]"
                  title={t('settings.cli.resetTitle')}
                >
                  {t('settings.cli.resetAuto')}
                </button>
              )}
            </>
          )}
        </div>
        {editingThis && error && <p className="pl-6 text-[11px] text-red-300">{error}</p>}
      </div>
    );
  };

  return (
    <BlockRow
      icon={<FolderCog size={13} className="text-[var(--text-muted)]" />}
      title={t('settings.cli.label')}
      desc={t('settings.cli.desc')}
    >
      <div className="flex flex-col gap-2">
        {installed.map(renderRow)}
        {uninstalled.length > 0 && (
          <div>
            <button
              onClick={() => setShowUninstalled((v) => !v)}
              className="flex items-center gap-1 rounded-lg px-1 py-0.5 text-[11px] text-[var(--text-muted)] transition-colors hover:text-[var(--text)]"
            >
              {showUninstalled ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
              {t('settings.cli.uninstalled', { n: uninstalled.length })}
            </button>
            {showUninstalled && (
              <div className="mt-2 flex flex-col gap-2 border-l border-[var(--border)] pl-2.5">
                {uninstalled.map(renderRow)}
              </div>
            )}
          </div>
        )}
        {agents.query.isLoading && (
          <Loader2 size={13} className="animate-spin text-[var(--text-muted)]" />
        )}
      </div>
    </BlockRow>
  );
}

/** 快问 AI 直连：OpenAI 兼容 base_url / 模型名 / API Key（各自失焦即存） */
function AiDirectBlock() {
  const { patchAsync } = usePatchSettings();
  const { data: settings } = useSettings();
  const { t } = useI18n();
  const ai = settings?.ai;
  return (
    <BlockRow
      icon={<KeyRound size={13} className="text-[var(--text-muted)]" />}
      title={t('settings.aiDirect.label')}
      desc={t('settings.aiDirect.desc', { url: ai?.base_url || t('settings.aiDirect.noAddress') })}
    >
      <div className="flex flex-col gap-2">
        <AutoSaveField
          value={ai?.base_url ?? ''}
          className="w-full"
          mono
          ariaLabel={t('settings.aiDirect.urlLabel')}
          placeholder={t('settings.aiDirect.urlPlaceholder')}
          onSave={(v) => patchAsync({ ai: { base_url: v } })}
        />
        <div className="flex gap-2">
          <AutoSaveField
            value={ai?.model ?? ''}
            className="flex-1"
            mono
            ariaLabel={t('settings.aiDirect.modelLabel')}
            placeholder={t('settings.aiDirect.modelPlaceholder')}
            onSave={(v) => patchAsync({ ai: { model: v } })}
          />
          <AutoSaveField
            value={ai?.api_key ?? ''}
            className="flex-1"
            mono
            password
            ariaLabel={t('settings.aiDirect.keyLabel')}
            placeholder={t('settings.aiDirect.keyPlaceholder')}
            onSave={(v) => patchAsync({ ai: { api_key: v || null } })}
          />
        </div>
      </div>
    </BlockRow>
  );
}

/** 天气城市：保存走 weatherSetCity（非 settings），保存后刷新天气缓存 */
function CityField() {
  const queryClient = useQueryClient();
  const { data: w } = useWeather();
  const { t } = useI18n();
  return (
    <AutoSaveField
      value={w?.city ?? ''}
      allowEmpty={false}
      ariaLabel={t('settings.weather.label')}
      placeholder={t('settings.weather.cityPlaceholder')}
      onSave={async (city) => {
        await commands.weatherSetCity(city);
        await queryClient.invalidateQueries({ queryKey: ['weather'] });
      }}
    />
  );
}
