# Interrupted operations

## Creating a new project

Generation and initial module composition take place in a hidden `.ferrule-create-*`
directory beside the destination. Only after they succeed does Ferrule install the
complete project tree with one atomic no-replace directory rename. It does not reserve
an empty destination or move individual entries. A destination that appears during
generation, including an empty folder or a dangling symlink, is preserved and creation fails.

The pinned `renamore` 0.3.2 implementation uses `renameat2(RENAME_NOREPLACE)` on Linux,
`renamex_np(RENAME_EXCL)` on macOS and `MoveFileExW` without replacement flags on Windows.
Unsupported OS/filesystem combinations fail closed; no non-atomic fallback is used.
See [the upstream operation contract](https://docs.rs/renamore/0.3.2/renamore/fn.rename_exclusive.html).

For process interruption at the final move, the destination is either absent or a complete
tree. This is a filesystem visibility guarantee, not a promise of power-loss durability.
If the process stops after the move but before reporting success, inspect the existing
project rather than retrying into it. Module previews can inspect a complete managed app.

A killed generator/Ferrule process can leave its hidden staging directory. Confirm the
process has stopped before inspecting or manually removing that exact directory. Ferrule
does not automatically resume it or remove abandoned staging folders. Previous versions'
incomplete destination folders still need manual review; `recover` cannot repair them.

The offline suite exercises complete-tree movement, existing empty/populated directories
and files, competing creators, Unix symlink targets and process termination immediately
before/after the rename. It does not inject a power failure or kill a kernel syscall midway.
`sh scripts/creation-smoke.sh` additionally runs the actual upstream generator and checks
ownership, expected project entries and refusal to recreate the same destination.

## Interrupted module writes

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
- Project creation uses the whole-directory operation described above, outside this
  journal. Generator/package-manager writes during `sync` and older incomplete
  creation destinations are not repaired by module recovery.

The offline suite covers partial writes, completed writes with a remaining journal,
ordinary second-write failure, interrupted rollback, edited-file conflicts,
stale locks, invalid hashes, duplicate keys, reserved-path aliases and symlinks.
