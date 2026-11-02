import type { Value } from "./index.js";

/** Platform binding. Implemented by the native runtime, never by bundled code.
 * SDK capability helpers use this port; operation access is checked by the host.
 */
export async function call(_operation: string, _args: Value[]): Promise<Value> {
  // Author tools may import SDK helpers in Node, but cannot perform privileged IO.
  // The bundler keeps this module external; QuickJS installs its native export.
  throw new Error("MeowCore script host bindings are unavailable outside the script runtime");
}
