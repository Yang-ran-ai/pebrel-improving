# Pebrel ↔ DeepSeek Harness bridge

This is a local DeepSeek Harness bundle, not a DeepSeek model-provider adapter. It subscribes to Harness's documented `session/event` feed and forwards lifecycle-only records to Pebrel's existing `pebrel-hook` helper.

Mapped events:

| Harness session event | Pebrel hook event |
| --- | --- |
| `user/message` | `UserPromptSubmit` |
| `tool/call` | `PreToolUse` |
| `tool/result` | `PostToolUse` |
| `turn/end` | `Stop` |

The bridge intentionally does not forward message bodies, tool arguments, or tool results. It requires the Harness Desktop process to inherit Pebrel's private hook-pipe environment and an absolute `PEBREL_HOOK_EXE` path. Without that host context it remains dormant.

Install this local bundle into the initialized Desktop profile with:

```powershell
dsh plugin --profile desktop add <persistent-path-to-this-directory>
```

Restart DeepSeek Harness after adding or removing the bundle. Do not install from a temporary directory: the Desktop profile records the local package path.

The Pebrel side must separately add `deepseek-harness` to the helper's source allowlist and parse it as a Claude-style lifecycle payload before this bridge can deliver events. The Pebrel Agents UI must also launch Harness with the per-workspace hook environment; installing the bundle alone is not a complete integration.
