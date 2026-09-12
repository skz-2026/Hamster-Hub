import { Archive, Code2, File as FileIcon, FileText, Film, Image as ImageIcon, Music } from 'lucide-react';

/** 文件类型 → 彩色图标（Spotlight / 最近文件共用） */
export function FileKindIcon({ kind }: { kind: string }) {
  const map: Record<string, { icon: typeof FileIcon; cls: string }> = {
    图片: { icon: ImageIcon, cls: 'bg-emerald-600' },
    视频: { icon: Film, cls: 'bg-purple-600' },
    音频: { icon: Music, cls: 'bg-pink-600' },
    压缩包: { icon: Archive, cls: 'bg-amber-600' },
    文档: { icon: FileText, cls: 'bg-sky-600' },
    代码: { icon: Code2, cls: 'bg-teal-600' },
    字体: { icon: FileText, cls: 'bg-slate-500' },
  };
  const { icon: Icon, cls } = map[kind] ?? { icon: FileIcon, cls: 'bg-slate-600' };
  return (
    <span className={`squircle grid size-8 shrink-0 place-items-center text-white ${cls}`}>
      <Icon size={15} />
    </span>
  );
}
