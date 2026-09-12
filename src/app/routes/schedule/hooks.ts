import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands, type CountdownCustom, type Note } from '@/shared/lib/ipc';

export function useNotes() {
  const qc = useQueryClient();
  const query = useQuery({ queryKey: ['note', 'list'], queryFn: () => commands.noteList() });
  const invalidate = () => qc.invalidateQueries({ queryKey: ['note'] });

  const create = useMutation({
    mutationFn: (content: string) => commands.noteCreate(content),
    onSuccess: invalidate,
  });
  const update = useMutation({
    mutationFn: ({ id, content }: { id: number; content: string }) =>
      commands.noteUpdate(id, content),
    onSuccess: invalidate,
  });
  const togglePin = useMutation({
    mutationFn: ({ id, pinned }: { id: number; pinned: boolean }) =>
      commands.noteTogglePin(id, pinned),
    onSuccess: invalidate,
  });
  const remove = useMutation({
    mutationFn: (id: number) => commands.noteDelete(id),
    onSuccess: invalidate,
  });
  return { query, create, update, togglePin, remove };
}

export function useCountdownCustom() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: ['countdown', 'custom'],
    queryFn: () => commands.countdownCustomList(),
  });
  const invalidate = () => qc.invalidateQueries({ queryKey: ['countdown'] });

  const create = useMutation({
    mutationFn: (v: { title: string; targetDate: string; emoji?: string }) =>
      commands.countdownCustomCreate(v.title, v.targetDate, v.emoji ?? null),
    onSuccess: invalidate,
  });
  const remove = useMutation({
    mutationFn: (id: number) => commands.countdownCustomDelete(id),
    onSuccess: invalidate,
  });
  return { query, create, remove };
}
