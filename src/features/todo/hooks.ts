import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, type Todo } from '@/shared/lib/ipc';

export function useTodos() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: ['todo', 'list'],
    queryFn: () => commands.todoList(),
  });

  const invalidate = () => qc.invalidateQueries({ queryKey: ['todo'] });

  const create = useMutation({
    mutationFn: (content: string) => commands.todoCreate(content),
    onSuccess: invalidate,
  });
  const toggle = useMutation({
    mutationFn: ({ id, done }: { id: number; done: boolean }) =>
      commands.todoToggle(id, done),
    onSuccess: invalidate,
  });
  const remove = useMutation({
    mutationFn: (id: number) => commands.todoDelete(id),
    onSuccess: invalidate,
  });

  return { query, create, toggle, remove };
}
