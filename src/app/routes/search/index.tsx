import { Search } from 'lucide-react';
import { StubRoute } from '@/shared/components/StubRoute';

export default function SearchPage() {
  return (
    <StubRoute
      icon={Search}
      title="搜索"
      milestone="M1"
      features={['Alt+Space 全局热键', '应用 / 文件 / 网页', '拼音与首字母模糊匹配', '常用优先排序', '问 AI（M2）']}
    />
  );
}
