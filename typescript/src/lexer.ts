/**
 * Natsuzora Lexer
 *
 * Tokenizes template source into a stream of tokens.
 * Functional implementation without classes.
 */

import { LexerError } from "./errors.ts";
import { Token, TokenType, createToken, KEYWORDS } from "./token.ts";
import { isIdentStart, isIdentCont } from "./validator.ts";

const OPEN = "{[";
const CLOSE = "]}";

interface LexerState {
  source: string;
  pos: number;
  line: number;
  column: number;
  tokens: Token[];
  insideTag: boolean;
  atTagStart: boolean;
  afterBang: boolean;
  stripAfterClose: boolean;
}

function createState(source: string): LexerState {
  return {
    source,
    pos: 0,
    line: 1,
    column: 1,
    tokens: [],
    insideTag: false,
    atTagStart: false,
    afterBang: false,
    stripAfterClose: false,
  };
}

function eof(state: LexerState): boolean {
  return state.pos >= state.source.length;
}

function currentChar(state: LexerState): string | null {
  if (eof(state)) return null;
  return state.source[state.pos];
}

function peekChar(state: LexerState): string | null {
  if (state.pos + 1 >= state.source.length) return null;
  return state.source[state.pos + 1];
}

function advance(state: LexerState): string {
  const char = currentChar(state)!;
  state.pos++;
  if (char === "\n") {
    state.line++;
    state.column = 1;
  } else {
    state.column++;
  }
  return char;
}

function match(state: LexerState, str: string): boolean {
  return state.source.slice(state.pos, state.pos + str.length) === str;
}

function matchWord(state: LexerState, word: string): boolean {
  return state.source.slice(state.pos, state.pos + word.length) === word;
}

function isWhitespace(char: string | null): boolean {
  if (char === null) return false;
  return char === " " || char === "\t" || char === "\r" || char === "\n";
}

function addToken(state: LexerState, type: TokenType, value: string | null, line: number, column: number): void {
  state.tokens.push(createToken(type, value, line, column));
}

function skipLeadingWhitespaceAndNewline(state: LexerState): void {
  // First, skip any whitespace/tabs before the newline (rest of current line)
  let lookahead = 0;
  while (state.pos + lookahead < state.source.length) {
    const char = state.source[state.pos + lookahead];
    if (char === "\n") break;
    if (char !== " " && char !== "\t" && char !== "\r") return;
    lookahead++;
  }

  // Skip the whitespace before newline
  for (let i = 0; i < lookahead; i++) {
    advance(state);
  }

  // Skip the newline itself
  if (currentChar(state) === "\n") {
    advance(state);

    // Skip leading whitespace on the next line
    while (currentChar(state) === " " || currentChar(state) === "\t") {
      advance(state);
    }
  }
}

function stripTrailingWhitespaceFromLastText(state: LexerState): void {
  let lastTextIdx = -1;
  for (let i = state.tokens.length - 1; i >= 0; i--) {
    if (state.tokens[i].type === "text") {
      lastTextIdx = i;
      break;
    }
  }
  if (lastTextIdx === -1) return;

  const textToken = state.tokens[lastTextIdx];
  const value = textToken.value!;

  const newlinePos = value.lastIndexOf("\n");
  if (newlinePos !== -1) {
    const suffix = value.slice(newlinePos + 1);
    if (!/^[ \t]*$/.test(suffix)) return;

    const newValue = value.slice(0, newlinePos + 1);
    state.tokens[lastTextIdx] = createToken("text", newValue, textToken.line, textToken.column);
  } else {
    if (!/^[ \t]*$/.test(value)) return;
    state.tokens.splice(lastTextIdx, 1);
  }
}

function skipComment(state: LexerState, startLine: number, startColumn: number): void {
  advance(state); // %

  while (!eof(state) && !match(state, CLOSE)) {
    advance(state);
  }

  if (eof(state)) {
    throw new LexerError("Unclosed comment", startLine, startColumn);
  }

  advance(state); // ]
  advance(state); // }
}

function consumeOpen(state: LexerState): void {
  const startLine = state.line;
  const startColumn = state.column;
  advance(state); // {
  advance(state); // [

  if (currentChar(state) === "%") {
    skipComment(state, startLine, startColumn);
    return;
  }

  if (currentChar(state) === "{") {
    advance(state); // {
    if (!match(state, CLOSE)) {
      throw new LexerError("Expected ']}' after '{[{'", state.line, state.column);
    }
    advance(state); // ]
    advance(state); // }
    addToken(state, "text", "{[", startLine, startColumn);
    return;
  }

  if (currentChar(state) === "-") {
    advance(state); // -
    stripTrailingWhitespaceFromLastText(state);
  }

  addToken(state, "open", OPEN, startLine, startColumn);
  state.insideTag = true;
  state.atTagStart = true;
}

function tokenizeText(state: LexerState): void {
  const startLine = state.line;
  const startColumn = state.column;
  let text = "";

  if (state.stripAfterClose) {
    state.stripAfterClose = false;
    skipLeadingWhitespaceAndNewline(state);
  }

  while (!eof(state) && !match(state, OPEN)) {
    text += advance(state);
  }

  if (text.length > 0) {
    addToken(state, "text", text, startLine, startColumn);
  }

  if (!eof(state)) {
    consumeOpen(state);
  }
}

function consumeClose(state: LexerState): void {
  const startLine = state.line;
  const startColumn = state.column;
  advance(state); // ]
  advance(state); // }
  addToken(state, "close", CLOSE, startLine, startColumn);
  state.insideTag = false;
}

function addSingleCharToken(state: LexerState, type: TokenType): void {
  const line = state.line;
  const column = state.column;
  const value = advance(state);
  addToken(state, type, value, line, column);
}

function skipWhitespaceWithToken(state: LexerState): void {
  if (!isWhitespace(currentChar(state))) return;

  const startLine = state.line;
  const startColumn = state.column;
  let value = "";

  while (isWhitespace(currentChar(state))) {
    value += advance(state);
  }

  addToken(state, "whitespace", value, startLine, startColumn);
}

function checkNoWhitespaceBeforeSpecialChars(state: LexerState): void {
  if (!state.atTagStart) return;
  if (!isWhitespace(currentChar(state))) return;

  let lookahead = 0;
  while (isWhitespace(state.source[state.pos + lookahead])) {
    lookahead++;
  }

  const nextChar = state.source[state.pos + lookahead];
  if (nextChar === "#" || nextChar === "/" || nextChar === "!") {
    throw new LexerError(
      `Whitespace not allowed before '${nextChar}' after tag open`,
      state.line,
      state.column
    );
  }
}

function tokenizeIdentifier(state: LexerState): void {
  const startLine = state.line;
  const startColumn = state.column;
  let value = "";

  while (isIdentCont(currentChar(state))) {
    value += advance(state);
  }

  const type = KEYWORDS[value] || "ident";
  addToken(state, type, value, startLine, startColumn);
}

function tokenizeIncludeName(state: LexerState): void {
  const startLine = state.line;
  const startColumn = state.column;
  let value = "";

  value += advance(state); // /

  while (true) {
    if (!isIdentStart(currentChar(state))) break;

    value += advance(state);

    while (isIdentCont(currentChar(state))) {
      value += advance(state);
    }

    if (currentChar(state) !== "/" || !isIdentStart(peekChar(state))) {
      break;
    }

    value += advance(state); // /
  }

  addToken(state, "ident", value, startLine, startColumn);
}

function tokenizeBang(state: LexerState): void {
  const startLine = state.line;
  const startColumn = state.column;
  advance(state); // !

  skipWhitespaceWithToken(state);

  if (matchWord(state, "unsecure")) {
    for (let i = 0; i < 8; i++) advance(state);
    addToken(state, "bang_unsecure", "!unsecure", startLine, startColumn);
  } else if (matchWord(state, "include")) {
    for (let i = 0; i < 7; i++) advance(state);
    addToken(state, "bang_include", "!include", startLine, startColumn);
    state.afterBang = true;
  } else {
    throw new LexerError(
      "Expected 'unsecure' or 'include' after '!'",
      state.line,
      state.column
    );
  }
}

function tokenizeInsideTag(state: LexerState): void {
  checkNoWhitespaceBeforeSpecialChars(state);
  skipWhitespaceWithToken(state);
  const wasAtTagStart = state.atTagStart;
  state.atTagStart = false;

  if (eof(state)) return;

  if (match(state, CLOSE)) {
    consumeClose(state);
    return;
  }

  if (currentChar(state) === "-" && peekChar(state) === "]") {
    advance(state); // -
    state.stripAfterClose = true;
    consumeClose(state);
    return;
  }

  const char = currentChar(state);
  switch (char) {
    case "#":
      addSingleCharToken(state, "hash");
      break;
    case "/":
      if (state.afterBang) {
        tokenizeIncludeName(state);
        state.afterBang = false;
      } else {
        addSingleCharToken(state, "slash");
      }
      break;
    case "!":
      if (wasAtTagStart) {
        tokenizeBang(state);
      } else {
        addSingleCharToken(state, "exclamation");
      }
      break;
    case "?":
      addSingleCharToken(state, "question");
      break;
    case "=":
      addSingleCharToken(state, "equal");
      break;
    case ",":
      addSingleCharToken(state, "comma");
      break;
    case ".":
      addSingleCharToken(state, "dot");
      break;
    default:
      state.afterBang = false;
      if (currentChar(state) === "/") {
        tokenizeIncludeName(state);
      } else if (isIdentStart(currentChar(state))) {
        tokenizeIdentifier(state);
      } else {
        throw new LexerError(
          `Unexpected character: '${currentChar(state)}'`,
          state.line,
          state.column
        );
      }
  }
}

/**
 * Tokenize a template source string into tokens.
 */
export function tokenize(source: string): Token[] {
  const state = createState(source);

  while (!eof(state)) {
    if (state.insideTag) {
      tokenizeInsideTag(state);
    } else {
      tokenizeText(state);
    }
  }

  addToken(state, "eof", null, state.line, state.column);
  return state.tokens;
}
