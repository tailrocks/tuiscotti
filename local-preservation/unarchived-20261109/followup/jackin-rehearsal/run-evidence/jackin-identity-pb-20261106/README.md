# Jackin #1264 binary consumer rehearsal

Date: 2026-11-06. Disposable only; no commit or PR.

## Pins and isolation

- Live PR #1264 rechecked OPEN at `480ba8faab70e87d9c276da6ebd3985c8f92c258`.
- Its locked TCB source is `a901c275055091810ae0d0ac30ec5fc09ce595d9` (`jackin-console` and `junie-tui`); its Tuiscotti source is `d8d54155947f6303a41dfd6079b53c0abc16f29a`.
- Fresh consumer copy: `/private/tmp/tuiscotti-jackin-pb-migration-rehearsal-20261106`; fresh TCB copy at `tcb/` is checked out at the exact TCB pin. The local `.cargo/config.toml` patches only those copies and `/private/tmp/tuiscotti-goal-current-20261106/crates/tuiscotti`.
- Candidate test input `tcb/apps/jackin/tests/functional/baselines/functional/jackin_launch_reconnect.tui.pb`, SHA-256 `24322d73987166a5c8a6aa58cb1d3316ec0ad8b058e1fe144eadf42e17a7bcb7`. This came from the prior disposable mechanics smoke. That smoke lacks human-review provenance; the file is only test input here, not an approved baseline. No accept command ran.
- The candidate source snapshot `SHA256SUMS` check passed before this rehearsal. No shared checkout, live active baseline root, public pin, PR head, or published ref changed.

## Throwaway consumer patch

`crates/jackin-testkit/tests/identity/coverage.rs` scans `.tui.pb` filenames. Its unchanged-copy helper reads binary bytes into a temporary root under the `.tui.pb` name. The former JSON mutation fixture was replaced with a semantic binary mutation: `Sample::from_bytes`, public `SnapshotCell`/`SnapshotParts`/`TerminalSnapshot::try_new`, alter the first cell foreground, then `Sample::state(...).to_bytes()`. It retains the old assertion for exactly one changed cell at `(0, 0)` with field `fg`; it does not parse or convert JSON.

The source patch SHA-256 is `25e49d23d5244fccfa0ff0e755a7fed291e341c2f12899d78047f4f3103986e7`.

## Commands and evidence

All Cargo commands used MBX with `MISE_CI=1`.

```sh
cd /private/tmp/tuiscotti-jackin-pb-migration-rehearsal-20261106
MISE_CI=1 mbx nextest run --offline -p jackin-testkit --test identity \
  -E 'test(=coverage::an_unchanged_approved_copy_passes_the_journey_check) | test(=coverage::a_one_cell_colour_change_fails_the_journey_check)' \
  --no-capture
```

Exit `0`, 2 tests passed. The mutated binary sample produced the existing semantic assertion: one changed cell at `(0,0)`, `fields: ["fg"]`. Log SHA-256 `dc2d257add64cc82098ee36863445129e714cfdd56c984dccd7e6ffbbb27834d`; exit file SHA-256 `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`.

```sh
cd /private/tmp/tuiscotti-jackin-pb-migration-rehearsal-20261106
MISE_CI=1 mbx nextest run --offline -p jackin-testkit --test identity \
  -E 'test(=coverage::every_approved_file_is_compared_by_one_journey)' \
  --no-capture
```

Expected exit `100`: 55 driven visual IDs vs 1 staged PB ID. Log SHA-256 `bdc58c5e34b4afd4f9f59c3e9176614a0592a5f171484f764152774c7212ee8d`; exit file SHA-256 `eea8254c7500ba3de996aa8ad6af399183f04e17d4a8102fde539dbc93a90012`.

The 54 IDs with no binary sample in this one-sample rehearsal are:

```text
jackin_account_form_pending  jackin_account_remove  jackin_close_pane_codex
jackin_close_pane_shell  jackin_close_tab  jackin_docker_no_engine
jackin_docker_setting  jackin_doctor  jackin_editor_discard  jackin_first_use_empty
jackin_fixed_account_disable_refused  jackin_fixed_account_edit_env
jackin_fixed_account_form  jackin_fixed_account_form_1password
jackin_fixed_account_form_env  jackin_fixed_account_form_pending
jackin_fixed_account_form_refused  jackin_fixed_account_menu
jackin_fixed_browser_read_only  jackin_fixed_browser_url
jackin_fixed_browser_url_refused  jackin_fixed_editor_accounts
jackin_fixed_editor_general  jackin_fixed_editor_general_refused
jackin_fixed_editor_project  jackin_fixed_env_form  jackin_fixed_file_menu_export
jackin_fixed_launch_picker  jackin_fixed_palette_export  jackin_fixed_settings_agents
jackin_fixed_settings_commits  jackin_fixed_settings_env  jackin_fixed_settings_mounts
jackin_fixed_settings_trust  jackin_fixed_wizard_destination
jackin_fixed_wizard_workdir  jackin_help_ownership  jackin_launch_env_prompt
jackin_launch_failure  jackin_launch_openshell  jackin_manager_stop_dialog
jackin_pane_attaching  jackin_pane_detached  jackin_pane_exited
jackin_pane_failed  jackin_pane_info_title  jackin_pane_pending
jackin_pane_viewing  jackin_paste_confirm  jackin_role_binding
jackin_role_editor  jackin_role_not_allowed  jackin_settings_leave
jackin_workspace_save
```

## CI implication and limit

Jackin's generated CI has a separate `rust-jackin-testkit` job; `.velnor/config.toml` selects `cargo_nextest`. The identity integration target is compiled on macOS/Linux. Once its scanner and consumer expect `.tui.pb`, the job fails until all 55 required IDs have freshly produced and explicitly reviewed/approved binary samples at the selected pinned TCB revision. CI must only consume those files; this rehearsal does not add capture or approval behavior. 
This is a one-sample consumer mechanics rehearsal, not complete ID coverage, not human approval, and not evidence of an updated live pin or merged SHA.
