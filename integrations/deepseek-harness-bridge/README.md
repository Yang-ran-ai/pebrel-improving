# Pebrel ↔ DeepSeek Harness bridge

This is a local DeepSeek Harness bundle, not a DeepSeek model-provider adapter. It subscribes to Harness's documented `session/event` feed and forwards lifecycle-only records to Pebrel's existing `pebrel-hook` helper.

Mapped events:

| Harness session event | Pebrel hook event |
| --- | --- |
| `user/message` | `UserPromptSubmit` |
| `tool/result` | `PostToolUse` |
| `turn/end` | `Stop` |

The bridge intentionally does not forward message bodies, tool arguments, or tool results. It serializes forwarding per session so event order is preserved. It requires the Harness Desktop process to inherit Pebrel's private hook-pipe environment, `PEBREL_PANE_ID`, and an absolute `PEBREL_HOOK_EXE` path. Without that host context it remains dormant.

Install this local bundle into the initialized Desktop profile with:

```powershell
dsh plugin --profile desktop add <persistent-path-to-this-directory>
```

Restart DeepSeek Harness after adding or removing the bundle. Do not install from a temporary directory: the Desktop profile records the local package path.

The Pebrel helper source allowlist and parser support for `deepseek-harness` are included in the current repository patch. The Pebrel Agents UI still needs to detect the installed Desktop, install/remove this bundle, and launch Harness with the per-workspace hook environment; installing the bundle alone is not a complete integration.
