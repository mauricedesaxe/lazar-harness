import type { Plugin } from "@opencode-ai/plugin"

// Why: absolute-path resident under ~/.lazar-harness/bin, so one invocation path serves hooks, plugins, and CI with no PATH setup anywhere.
const BIN = `${process.env.HOME}/.lazar-harness/bin/harness-check`

type ToolArgs = { filePath?: unknown }

export const HarnessCheck: Plugin = async () => ({
  "tool.execute.after": async (input, output) => {
    const tool = input.tool
    if (tool !== "edit" && tool !== "write" && tool !== "apply_patch") return
    const filePath = (output.args as ToolArgs | undefined)?.filePath
    if (typeof filePath !== "string") return

    let code: number
    let stderr = ""
    try {
      const proc = Bun.spawn([BIN, "check", filePath], {
        stdout: "ignore",
        stderr: "pipe",
      })
      code = await proc.exited
      stderr = await new Response(proc.stderr).text()
    } catch {
      return
    }
    if (code === 2) {
      throw new Error(stderr || "harness-check blocked the edit")
    }
  },
})
