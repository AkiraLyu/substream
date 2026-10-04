import { message } from "./i18n.ts";

export function validateToken(token: string): void {
  if (!/^[a-fA-F0-9]{64}$/.test(token)) throw new Error(message("invalidToken"));
}
