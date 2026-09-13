/**
 * 密码箱 hooks：状态/列表查询 + 全部写操作；锁定事件统一失效查询（页面自动切锁屏）。
 */
import { useEffect, useRef } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  commands,
  events,
  type GenOptions,
  type VaultItem,
  type VaultItemInput,
} from '@/shared/lib/ipc';

export function useVaultStatus() {
  return useQuery({
    queryKey: ['vault', 'status'],
    queryFn: () => commands.vaultStatus(),
  });
}

/** 列表/搜索共用（空串 = 全量列表；由 Rust 侧 LIKE 命中标题/用户名/网址） */
export function useVaultItems(query: string) {
  const q = query.trim();
  return useQuery({
    queryKey: ['vault', 'items', q],
    queryFn: () => (q ? commands.vaultItemSearch(q) : commands.vaultItemList()),
  });
}

export function useVaultActions() {
  const qc = useQueryClient();
  const invalidate = () => qc.invalidateQueries({ queryKey: ['vault'] });

  const setup = useMutation({
    mutationFn: (v: { password: string; hint: string | null; autoLockSecs: number }) =>
      commands.vaultSetup(v.password, v.hint, v.autoLockSecs),
    onSuccess: invalidate,
  });
  const unlock = useMutation({
    mutationFn: (password: string) => commands.vaultUnlock(password),
    onSuccess: invalidate,
  });
  const lock = useMutation({
    mutationFn: () => commands.vaultLock(),
    onSuccess: invalidate,
  });
  const setAutoLock = useMutation({
    mutationFn: (secs: number) => commands.vaultSetAutoLock(secs),
    onSuccess: invalidate,
  });
  const changePassword = useMutation({
    mutationFn: (v: { oldPassword: string; newPassword: string; hint: string | null }) =>
      commands.vaultChangePassword(v.oldPassword, v.newPassword, v.hint),
    onSuccess: invalidate,
  });
  const create = useMutation({
    mutationFn: (input: VaultItemInput) => commands.vaultItemCreate(input),
    onSuccess: invalidate,
  });
  const update = useMutation({
    mutationFn: (v: { id: number; input: VaultItemInput }) =>
      commands.vaultItemUpdate(v.id, v.input),
    onSuccess: invalidate,
  });
  const toggleFavorite = useMutation({
    mutationFn: (v: { id: number; favorite: boolean }) =>
      commands.vaultItemToggleFavorite(v.id, v.favorite),
    onSuccess: invalidate,
  });
  const remove = useMutation({
    mutationFn: (id: number) => commands.vaultItemDelete(id),
    onSuccess: invalidate,
  });
  const reveal = useMutation({
    mutationFn: (id: number) => commands.vaultItemReveal(id),
  });
  const copyField = useMutation({
    mutationFn: (v: { id: number; field: 'password' | 'username' }) =>
      commands.vaultCopyField(v.id, v.field),
  });
  const generate = useMutation({
    mutationFn: (options: GenOptions) => commands.vaultPasswordGenerate(options),
  });

  return {
    setup,
    unlock,
    lock,
    setAutoLock,
    changePassword,
    create,
    update,
    toggleFavorite,
    remove,
    reveal,
    copyField,
    generate,
  };
}

/**
 * 后端锁定事件（手动/闲置看门狗）→ 失效状态查询；reason === 'idle' 时回调提醒。
 * 回调经 ref 转发，避免每次渲染重订阅。
 */
export function useVaultLockListener(onLocked?: (reason: string) => void) {
  const qc = useQueryClient();
  const cbRef = useRef(onLocked);
  cbRef.current = onLocked;
  useEffect(() => {
    const unlisten = events.vaultLocked.listen((e) => {
      qc.invalidateQueries({ queryKey: ['vault'] });
      cbRef.current?.(e.payload.reason);
    });
    return () => {
      unlisten.then((fn) => fn()).catch(console.error);
    };
  }, [qc]);
}
