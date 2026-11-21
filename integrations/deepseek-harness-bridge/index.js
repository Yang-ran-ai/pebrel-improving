import { spawn } from 'node:child_process'
import { isAbsolute } from 'node:path'

export const name = 'pebrel-deepseek-harness-bridge'

const EVENT_MAP = new Map([
  ['user/message', 'UserPromptSubmit'],
  ['tool/call', 'PreToolUse'],
  ['tool/result', 'PostToolUse'],
  ['turn/end', 'Stop'],
])

function makePayload(session, event) {
  const hookEvent = EVENT_MAP.get(event.type)
  if (!hookEvent) return undefined

  const data = event.data ?? {}
  const payload = {
    hook_event_name: hookEvent,
    session_id: String(session.id),
    event_id: `dsh-${String(session.id)}-${event.seq}`,
    bridge_sequence: Number(event.seq),
    occurred_at_ms: Number(event.time),
    cwd: typeof session.cwd === 'string' ? session.cwd : process.cwd(),
    source: 'deepseek-harness',
  }

  if (event.type === 'tool/call') {
    payload.tool_name = typeof data.name === 'string' ? data.name : 'unknown'
  } else if (event.type === 'tool/result') {
    payload.tool_name = typeof data.message?.name === 'string' ? data.message.name : 'unknown'
  }

  return payload
}

function forward(helperPath, session, event) {
  if (!isAbsolute(helperPath)) return
  if (!process.env.PEBREL_NOTIFY_PIPE && !process.env.NEBULA_NOTIFY_PIPE) return

  const payload = makePayload(session, event)
  if (!payload) return

  const child = spawn(helperPath, ['deepseek-harness'], {
    env: process.env,
    stdio: ['pipe', 'ignore', 'ignore'],
    windowsHide: true,
  })
  child.on('error', () => {})
  child.stdin.on('error', () => {})
  child.stdin.end(JSON.stringify(payload))
}

export function apply(ctx, config = {}) {
  const helperPath = typeof config.helperPath === 'string' ? config.helperPath : ''
  if (!isAbsolute(helperPath)) {
    ctx.logger.warn('pebrel-deepseek-harness-bridge: Pebrel hook helper path is unavailable; bridge is dormant')
    return
  }

  ctx.on('session/event', (session, event) => {
    try {
      forward(helperPath, session, event)
    } catch (error) {
      ctx.logger.warn(`pebrel-deepseek-harness-bridge: event forwarding failed: ${String(error)}`)
    }
  })
}
