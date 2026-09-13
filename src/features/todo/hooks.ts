import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, type Todo } from '@/shared/lib/ipc';

export interface NewTodo {
  content: string;
  dueAt: number | null;
  /** 有截止时间时默认开提醒 */
  remind: boolean;
}

export function useTodos() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: ['todo', 'list'],
    queryFn: () => commands.todoList(),
  });

  const invalidate = () => qc.invalidateQueries({ queryKey: ['todo'] });

  const create = useMutation({
    mutationFn: (v: NewTodo) =>
      commands.todoCreate(v.content, v.dueAt, v.dueAt != null && v.remind ? true : null),
    onSuccess: invalidate,
  });
  const toggle = useMutation({
    mutationFn: ({ id, done }: { id: number; done: boolean }) =>
      commands.todoToggle(id, done),
    onSuccess: invalidate,
  });
  const setDue = useMutation({
    mutationFn: ({ id, dueAt, remind }: { id: number; dueAt: number | null; remind: boolean }) =>
      commands.todoSetDue(id, dueAt, remind),
    onSuccess: invalidate,
  });
  const setRecur = useMutation({
    mutationFn: ({ id, recur }: { id: number; recur: string | null }) =>
      commands.todoSetRecur(id, recur),
    onSuccess: invalidate,
  });
  const remove = useMutation({
    mutationFn: (id: number) => commands.todoDelete(id),
    onSuccess: invalidate,
  });

  return { query, create, toggle, setDue, setRecur, remove };
}
