# Interrupted module writes

Before changing managed files, Ferrule persists `.ferrule-transaction.json` in the
project root. It contains original bytes and expected after-state hashes. Successful
installation removes it; ordinary write errors attempt the same rollback as recovery.
Pending journals block `add`, `add --dry-run` and `sync` until resolved.

## Recovering a stopped operation

1. Confirm the original Ferrule process has stopped and pause project editors.
   If `.ferrule.lock` remains, inspect it and remove only that stale lock after
   confirming no operation is running. Recovery never removes a pre-existing lock.
2. Run `ferrule recover --project PATH --dry-run`. This prints only paths needing
   restoration and leaves the journal, project files and locks unchanged.
3. Run `ferrule recover --project PATH` to restore original files, delete files
   newly created by the interrupted operation, and remove the journal. An empty
   preview can still mean all files are original and only journal cleanup remains.
4. Retry the original `add` command, inspect its result, then run `sync` and checks.

Recovery accepts either the original bytes or Ferrule's expected written bytes.
It validates all recorded targets before any restore. A later edit, deletion of an
original file, unsupported path, symlink, malformed journal or duplicate path stops
recovery for manual review. Preserve both your edits and the local journal while
resolving a conflict; another recovery attempt accepts already-restored files.
Only exact paths supported by the current embedded module catalog and configuration
writers are accepted. Case/separator aliases and arbitrary repository paths are refused.

## Limits and local data

- The journal contains source/configuration backups. Keep it local and exclude it
  from commits or support uploads. Treat downloaded or externally edited journals
  as untrusted; syntax/path validation does not authenticate their original contents.
- Recovery rolls back the whole recorded operation, including when every write
  finished but journal removal was interrupted. The ownership record is restored last.
- Restored files preserve bytes, not original filesystem permissions or timestamps.
  Empty directories created during the operation can remain.
- Pausing other writers is required. Filesystem checks do not coordinate with editors
  or hostile concurrent processes. No power-loss, filesystem-corruption or atomic
  multi-file-commit guarantee is made.
- Interrupted new-project final moves and generator/package-manager writes during
  `sync` are outside this journal. Existing destinations are still preserved.

The offline suite covers partial writes, completed writes with a remaining journal,
ordinary second-write failure, interrupted rollback, edited-file conflicts,
stale locks, invalid hashes, duplicate keys, reserved-path aliases and symlinks.
