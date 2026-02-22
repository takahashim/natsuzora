/**
 * Natsuzora Token Types
 */

export type TokenType =
  | "text"
  | "open"
  | "close"
  | "hash"
  | "slash"
  | "dot"
  | "equal"
  | "comma"
  | "question"
  | "exclamation"
  | "whitespace"
  | "ident"
  | "kw_if"
  | "kw_unless"
  | "kw_else"
  | "kw_each"
  | "kw_as"
  | "bang_unsecure"
  | "bang_include"
  | "eof";

export interface Token {
  type: TokenType;
  value: string | null;
  line: number;
  column: number;
}

export function createToken(
  type: TokenType,
  value: string | null,
  line: number,
  column: number
): Token {
  return { type, value, line, column };
}

/** Keywords that are recognized by the lexer */
export const KEYWORDS: Record<string, TokenType> = {
  if: "kw_if",
  unless: "kw_unless",
  else: "kw_else",
  each: "kw_each",
  as: "kw_as",
};

/** Reserved words that cannot be used as identifiers */
export const RESERVED_WORDS = new Set([
  "if",
  "unless",
  "else",
  "each",
  "as",
  "in",
  "of",
  "unsecure",
  "true",
  "false",
  "null",
  "include",
]);
