/** Vault copy (keys must match the zh-CN baseline dictionary) */
export const vault = {
  // —— Header ——
  'pages.vault.title': 'Vault',
  'pages.vault.subtitle': 'Passwords encrypted locally · master key lives in memory only',

  // —— Setup ——
  'pages.vault.setup.title': 'Create your vault',
  'pages.vault.setup.desc': 'The master password encrypts everything. It cannot be recovered and never leaves this device — remember it well.',
  'pages.vault.setup.master': 'Master password',
  'pages.vault.setup.masterPh': 'At least 8 characters',
  'pages.vault.setup.confirm': 'Confirm master password',
  'pages.vault.setup.confirmPh': 'Enter it again',
  'pages.vault.setup.hint': 'Hint (optional)',
  'pages.vault.setup.hintPh': 'e.g. the usual one + birthday',
  'pages.vault.setup.submit': 'Create vault',
  'pages.vault.setup.mismatch': 'The two passwords do not match',
  'pages.vault.setup.tooShort': 'Master password needs at least 8 characters',

  // —— Unlock ——
  'pages.vault.unlock.title': 'Vault is locked',
  'pages.vault.unlock.desc': 'Enter the master password to unlock',
  'pages.vault.unlock.master': 'Master password',
  'pages.vault.unlock.hint': 'Hint: {hint}',
  'pages.vault.unlock.submit': 'Unlock',
  'pages.vault.unlock.failed': 'Wrong master password',
  'pages.vault.unlock.idle': 'Locked automatically after inactivity',

  // —— List ——
  'pages.vault.list.searchPh': 'Search title / username / URL',
  'pages.vault.list.new': 'New',
  'pages.vault.list.emptyTitle': 'The vault is empty',
  'pages.vault.list.emptyDesc': 'Save your first password, then search and copy it here anytime',
  'pages.vault.list.emptySearch': 'No matching items',
  'pages.vault.list.count': '{count} items',
  'pages.vault.list.lock': 'Lock',

  // —— Auto lock ——
  'pages.vault.autolock.label': 'Auto lock',
  'pages.vault.autolock.60': '1 min',
  'pages.vault.autolock.300': '5 min',
  'pages.vault.autolock.900': '15 min',
  'pages.vault.autolock.1800': '30 min',
  'pages.vault.autolock.3600': '1 hour',

  // —— Item editor ——
  'pages.vault.item.title': 'Title',
  'pages.vault.item.titleRequired': 'Title is required',
  'pages.vault.item.titlePh': 'e.g. GitHub',
  'pages.vault.item.username': 'Username',
  'pages.vault.item.usernamePh': 'Email / phone / username',
  'pages.vault.item.url': 'URL',
  'pages.vault.item.urlPh': 'https://example.com',
  'pages.vault.item.password': 'Password',
  'pages.vault.item.notes': 'Notes',
  'pages.vault.item.notesPh': 'Recovery codes, linked phone, etc.',
  'pages.vault.item.favorite': 'Favorite',
  'pages.vault.item.newTitle': 'New password',
  'pages.vault.item.editTitle': 'Edit password',
  'pages.vault.item.save': 'Save',
  'pages.vault.item.cancel': 'Cancel',
  'pages.vault.item.delete': 'Delete',
  'pages.vault.item.deleteConfirm': 'Delete “{title}”? It goes to trash (coming soon).',
  'pages.vault.item.reveal': 'Show password',
  'pages.vault.item.hide': 'Hide password',

  // —— Copy ——
  'pages.vault.copy.password': 'Copy password',
  'pages.vault.copy.username': 'Copy username',
  'pages.vault.copy.done': 'Copied · clipboard clears in {secs}s',
  'pages.vault.copy.doneKeep': 'Copied',
  'pages.vault.copy.hint': 'Note: Windows clipboard history (Win+V) keeps copies — consider turning it off in system settings',

  // —— Generator ——
  'pages.vault.gen.title': 'Generate password',
  'pages.vault.gen.length': 'Length',
  'pages.vault.gen.upper': 'Uppercase',
  'pages.vault.gen.lower': 'Lowercase',
  'pages.vault.gen.digits': 'Digits',
  'pages.vault.gen.symbols': 'Symbols',
  'pages.vault.gen.ambiguous': 'Exclude look-alikes (I l 1 O 0)',
  'pages.vault.gen.refresh': 'Reroll',
  'pages.vault.gen.apply': 'Use',

  // —— Strength ——
  'pages.vault.strength.0': 'Weak',
  'pages.vault.strength.1': 'Fair',
  'pages.vault.strength.2': 'Moderate',
  'pages.vault.strength.3': 'Strong',
  'pages.vault.strength.4': 'Very strong',

  // —— Change master password ——
  'pages.vault.change.title': 'Change master password',
  'pages.vault.change.old': 'Current master password',
  'pages.vault.change.new': 'New master password',
  'pages.vault.change.confirm': 'Confirm new password',
  'pages.vault.change.submit': 'Apply change',
  'pages.vault.change.failed': 'Current master password is wrong',
  'pages.vault.change.mismatch': 'The two entries do not match',
} as const;
