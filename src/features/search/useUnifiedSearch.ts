/**
 * 统一检索 hook：应用路（app_search，拼音/首字母/usage 加权）+ 文件路（file_search，
 * FTS5 拼音双形态）。Spotlight 覆盖层与 /search 搜索页共用，保证两处检索语义一致。
 */
import { useQuery } from '@tanstack/react-query';
import { commands } from '@/shared/lib/ipc';

export function useUnifiedSearch(q: string, appLimit = 8, fileLimit = 6) {
  const query = q.trim().toLowerCase();
  const apps = useQuery({
    queryKey: ['apps', 'search', query, appLimit],
    queryFn: () => commands.appSearch(query, appLimit),
    enabled: query.length > 0,
    staleTime: 5_000,
  });
  const files = useQuery({
    queryKey: ['files', 'search', query, fileLimit],
    queryFn: () => commands.fileSearch(query, fileLimit),
    enabled: query.length > 0,
    staleTime: 5_000,
  });
  return { apps: apps.data ?? [], files: files.data ?? [] };
}
