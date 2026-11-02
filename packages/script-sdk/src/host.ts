import type { Value } from "./index.js";

/** Platform binding. Implemented by the native runtime, never by bundled code.
 * SDK capability helpers use this port; operation access is checked by the host.
 */
export declare function call(operation: string, args: Value[]): Promise<Value>;
