/**
 * AgentAvatar：Agent 品牌头像（对齐 molto AgentAvatar 设计）——
 * 官方品牌图标优先（本地打包 SVG，按品牌色着色），
 * 无官方图标的 Agent（及未知 id）回退品牌色字母头像。
 */
import { useMemo } from 'react';

export interface AgentAvatarProps {
  agentId: string;
  /** 边长 px（默认 18，与侧栏行高匹配） */
  size?: number;
  /** 悬停提示（纯图标处标注 agent 名，替代被图标替代的文字） */
  title?: string;
}

/**
 * 品牌色取官方视觉规范或社区惯例（simple-icons / 公开品牌素材 CC0）；
 * icon 指向 assets/agents/ 下的 SVG 文件名。新接入 Agent 时这里同步登记。
 */
const BRAND: Record<string, { color: string; label: string; icon?: string }> = {
  // 专设适配器（接管配置面 + 运行时面）
  claude: { color: '#D97757', label: 'C', icon: 'claude' },
  codex: { color: '#10A37F', label: 'X', icon: 'openai' },
  gemini: { color: '#8E75B2', label: 'G', icon: 'googlegemini' },
  zcode: { color: '#3859DD', label: 'Z' },
  // ACP 通道型适配器（仅接对话通道）
  opencode: { color: '#C9A15F', label: 'O', icon: 'opencode' },
  codebuddy: { color: '#6C4DFF', label: 'CB', icon: 'codebuddy' },
  cursor: { color: '#8A857C', label: 'Cu', icon: 'cursor' },
  kimi: { color: '#1E88E5', label: 'K', icon: 'kimi' },
  copilot: { color: '#1F883D', label: 'Co', icon: 'copilot' },
  qwen: { color: '#615CED', label: 'Q', icon: 'qwen' },
  auggie: { color: '#5B47E0', label: 'Au', icon: 'auggie' },
  cline: { color: '#FBBD35', label: 'Cl', icon: 'cline' },
  goose: { color: '#FF7F32', label: 'Go', icon: 'goose' },
  kilo: { color: '#FFB800', label: 'Ki', icon: 'kilo' },
  vibe: { color: '#FF700A', label: 'V', icon: 'vibe' },
  droid: { color: '#6E56CF', label: 'D', icon: 'droid' },
  devin: { color: '#0071F2', label: 'De', icon: 'devin' },
  junie: { color: '#FE5196', label: 'J', icon: 'junie' },
  grok: { color: '#1F1F1F', label: 'Gr', icon: 'grok' },
  cortex: { color: '#29B5E8', label: 'Cx', icon: 'cortex' },
  poolside: { color: '#2563EB', label: 'Po', icon: 'poolside' },
  stakpak: { color: '#8B5CF6', label: 'St', icon: 'stakpak' },
  'fast-agent': { color: '#0EA5E9', label: 'Fa', icon: 'fast-agent' },
  amp: { color: '#1F1F1F', label: 'A', icon: 'amp' },
  glm: { color: '#4475FF', label: 'G', icon: 'glm' },
  pi: { color: '#1F1F1F', label: 'P', icon: 'pi' },
  // 暂无官方图标（simple-icons 未收录），按规范回退品牌色字母头像；后续补 SVG 时加 icon 字段
  openhands: { color: '#1F1F1F', label: 'OH' },
  kiro: { color: '#FF9900', label: 'K' },
  qoder: { color: '#1F1F1F', label: 'Q' },
  minimax: { color: '#1F1F1F', label: 'MM' },
};

const FALLBACK = { color: 'linear-gradient(135deg, #8A857C, #5C5850)', label: '' };

const ICONS = import.meta.glob('../../assets/agents/*.svg', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>;

export default function AgentAvatar({ agentId, size = 18, title }: AgentAvatarProps) {
  const brand = BRAND[agentId] ?? { ...FALLBACK, label: agentId.charAt(0).toUpperCase() };
  // 图标来自本地打包资源，非用户输入
  const iconSvg = useMemo(
    () => (brand.icon ? ICONS[`../../assets/agents/${brand.icon}.svg`] : undefined),
    [brand.icon],
  );

  return (
    <span
      title={title}
      className={`inline-grid shrink-0 place-items-center overflow-hidden rounded-[28%] ${
        iconSvg ? 'bg-[var(--bg,#26242b)] ring-1 ring-white/10' : 'text-white'
      }`}
      style={{
        width: size,
        height: size,
        fontSize: Math.round(size * 0.44),
        ...(iconSvg ? {} : { background: brand.color }),
      }}
    >
      {iconSvg ? (
        <span
          className="inline-flex w-[68%] h-[68%] [&>svg]:w-full [&>svg]:h-full [&>svg]:fill-current"
          style={{ color: brand.color }}
          dangerouslySetInnerHTML={{ __html: iconSvg }}
        />
      ) : (
        <span className="font-semibold leading-none">{brand.label}</span>
      )}
    </span>
  );
}
