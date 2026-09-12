import { FolderOpen } from 'lucide-react';
import { StubRoute } from '@/shared/components/StubRoute';

export default function FilesPage() {
  return (
    <StubRoute
      icon={FolderOpen}
      title="文件"
      milestone="M2"
      features={['本地文件索引', '类型筛选', '时间 / 名称排序', '拼音搜索']}
    />
  );
}
