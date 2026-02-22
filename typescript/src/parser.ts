/**
 * Natsuzora Parser
 *
 * Parses tokens into an AST.
 * Functional implementation without classes.
 */

import { ParseError, ReservedWordError } from "./errors.ts";
import { Token, TokenType, RESERVED_WORDS } from "./token.ts";
import {
  Node,
  Template,
  Path,
  IncludeArg,
  VariableModifier,
  createPath,
  textNode,
  variableNode,
  ifBlockNode,
  unlessBlockNode,
  eachBlockNode,
  unsecureOutputNode,
  includeNode,
  template,
} from "./ast.ts";
import { validateIncludeName } from "./validator.ts";

interface ParserState {
  tokens: Token[];
  pos: number;
}

function createState(tokens: Token[]): ParserState {
  // Filter out whitespace tokens for easier parsing
  return {
    tokens: tokens.filter((t) => t.type !== "whitespace"),
    pos: 0,
  };
}

function current(state: ParserState): Token {
  return state.tokens[state.pos] || { type: "eof", value: null, line: 0, column: 0 };
}

/**
 * Get the value of a token, throwing if null.
 * Use this after verifying the token type guarantees a value.
 */
function tokenValue(token: Token): string {
  if (token.value === null) {
    throw new ParseError(
      `Expected token value for ${token.type}`,
      token.line,
      token.column
    );
  }
  return token.value;
}

function peek(state: ParserState): Token | null {
  return state.tokens[state.pos + 1] || null;
}

function peekAt(state: ParserState, offset: number): Token | null {
  return state.tokens[state.pos + offset] || null;
}

function advance(state: ParserState): void {
  state.pos++;
}

function eof(state: ParserState): boolean {
  return state.pos >= state.tokens.length || current(state).type === "eof";
}

function expect(state: ParserState, type: TokenType): void {
  const token = current(state);
  if (token.type !== type) {
    throw new ParseError(`Expected ${type}, got ${token.type}`, token.line, token.column);
  }
  advance(state);
}

function expectKeyword(state: ParserState, type: TokenType, name: string): void {
  const token = current(state);
  if (token.type !== type) {
    throw new ParseError(`Expected '${name}', got ${token.type}`, token.line, token.column);
  }
  advance(state);
}

function validatePathSegment(name: string, line: number, column: number): void {
  if (name.startsWith("_")) {
    throw new ParseError(`Identifier '${name}' cannot start with underscore`, line, column);
  }
  if (RESERVED_WORDS.has(name)) {
    throw new ReservedWordError(name, line, column);
  }
}

function validateEachVariable(name: string, line: number, column: number): void {
  if (name.startsWith("_")) {
    throw new ParseError(`Identifier '${name}' cannot start with underscore`, line, column);
  }
  if (RESERVED_WORDS.has(name)) {
    throw new ReservedWordError(name, line, column);
  }
}

function parsePath(state: ParserState): Path {
  const segments: string[] = [];
  const firstToken = current(state);

  if (firstToken.type !== "ident") {
    throw new ParseError(`Expected identifier, got ${firstToken.type}`, firstToken.line, firstToken.column);
  }

  const firstName = tokenValue(firstToken);
  validatePathSegment(firstName, firstToken.line, firstToken.column);
  segments.push(firstName);
  advance(state);

  while (current(state).type === "dot") {
    advance(state);
    const segmentToken = current(state);
    if (segmentToken.type !== "ident") {
      throw new ParseError(`Expected identifier after '.', got ${segmentToken.type}`, segmentToken.line, segmentToken.column);
    }
    const segment = tokenValue(segmentToken);
    validatePathSegment(segment, segmentToken.line, segmentToken.column);
    segments.push(segment);
    advance(state);
  }

  return createPath(segments);
}

function parseModifier(state: ParserState): VariableModifier {
  if (current(state).type === "question") {
    advance(state);
    return "nullable";
  }
  if (current(state).type === "exclamation") {
    advance(state);
    return "required";
  }
  return null;
}

function parseVariable(state: ParserState): Node {
  const path = parsePath(state);
  const modifier = parseModifier(state);
  expect(state, "close");
  return variableNode(path, modifier);
}

function parseUnsecureOutput(state: ParserState): Node {
  expect(state, "bang_unsecure");
  const path = parsePath(state);
  expect(state, "close");
  return unsecureOutputNode(path);
}

function parseInclude(state: ParserState): Node {
  expect(state, "bang_include");

  const nameToken = current(state);
  const nameValue = nameToken.type === "ident" ? tokenValue(nameToken) : null;
  if (nameToken.type !== "ident" || !nameValue?.startsWith("/")) {
    throw new ParseError(`Expected include name starting with '/', got ${nameValue}`, nameToken.line, nameToken.column);
  }

  const name = nameValue;
  validateIncludeName(name, nameToken.line, nameToken.column);
  advance(state);

  const args: IncludeArg[] = [];

  while (current(state).type === "ident" && !tokenValue(current(state)).startsWith("/")) {
    const keyToken = current(state);
    const key = tokenValue(keyToken);

    if (key.startsWith("_")) {
      throw new ParseError(`Identifier '${key}' cannot start with underscore`, keyToken.line, keyToken.column);
    }

    if (args.some((a) => a.key === key)) {
      throw new ParseError(`Duplicate include argument: ${key}`, keyToken.line, keyToken.column);
    }

    advance(state);
    expect(state, "equal");
    const value = parsePath(state);
    args.push({ key, value });
  }

  expect(state, "close");
  return includeNode(name, args);
}

function parseIfBlock(state: ParserState): Node {
  expect(state, "kw_if");
  const condition = parsePath(state);
  expect(state, "close");

  const thenBranch = parseNodes(state);

  let elseBranch: Node[] | null = null;

  if (current(state).type === "open") {
    const next = peek(state);
    if (next?.type === "hash") {
      const afterHash = peekAt(state, 2);
      if (afterHash?.type === "kw_else") {
        expect(state, "open");
        expect(state, "hash");
        expect(state, "kw_else");
        expect(state, "close");
        elseBranch = parseNodes(state);
      }
    }
  }

  expect(state, "open");
  expect(state, "slash");
  expectKeyword(state, "kw_if", "if");
  expect(state, "close");

  return ifBlockNode(condition, thenBranch, elseBranch);
}

function parseUnlessBlock(state: ParserState): Node {
  expect(state, "kw_unless");
  const condition = parsePath(state);
  expect(state, "close");

  const body = parseNodes(state);

  expect(state, "open");
  expect(state, "slash");
  expectKeyword(state, "kw_unless", "unless");
  expect(state, "close");

  return unlessBlockNode(condition, body);
}

function parseEachBlock(state: ParserState): Node {
  expect(state, "kw_each");
  const collection = parsePath(state);
  expect(state, "kw_as");

  const itemToken = current(state);
  if (itemToken.type !== "ident") {
    throw new ParseError(`Expected identifier for each item, got ${itemToken.type}`, itemToken.line, itemToken.column);
  }
  const itemName = tokenValue(itemToken);
  validateEachVariable(itemName, itemToken.line, itemToken.column);
  advance(state);

  expect(state, "close");

  const body = parseNodes(state);

  expect(state, "open");
  expect(state, "slash");
  expectKeyword(state, "kw_each", "each");
  expect(state, "close");

  return eachBlockNode(collection, itemName, body);
}

function parseBlockOpen(state: ParserState): Node {
  expect(state, "hash");
  const token = current(state);

  switch (token.type) {
    case "kw_if":
      return parseIfBlock(state);
    case "kw_unless":
      return parseUnlessBlock(state);
    case "kw_each":
      return parseEachBlock(state);
    default:
      throw new ParseError(`Unknown block type: ${token.type}`, token.line, token.column);
  }
}

function isKeywordToken(type: string): boolean {
  return type.startsWith("kw_");
}

function keywordToName(type: string): string {
  // "kw_if" -> "if", "kw_each" -> "each", etc.
  return type.slice(3);
}

function parseTag(state: ParserState): Node {
  expect(state, "open");
  const token = current(state);

  switch (token.type) {
    case "hash":
      return parseBlockOpen(state);
    case "bang_unsecure":
      return parseUnsecureOutput(state);
    case "bang_include":
      return parseInclude(state);
    case "ident":
      return parseVariable(state);
    default:
      // Check if it's a keyword being used as identifier
      if (isKeywordToken(token.type)) {
        throw new ReservedWordError(keywordToName(token.type), token.line, token.column);
      }
      throw new ParseError(`Unexpected token in tag: ${token.type}`, token.line, token.column);
  }
}

function parseNode(state: ParserState): Node | null {
  const token = current(state);

  switch (token.type) {
    case "text":
      advance(state);
      return textNode(tokenValue(token));
    case "open":
      return parseTag(state);
    case "eof":
      return null;
    default:
      throw new ParseError(`Unexpected token: ${token.type}`, token.line, token.column);
  }
}

function parseNodes(state: ParserState): Node[] {
  const nodes: Node[] = [];

  while (!eof(state)) {
    const token = current(state);

    // Check for block close or else
    if (token.type === "open") {
      const next = peek(state);
      if (next?.type === "slash" || next?.type === "kw_else") {
        break;
      }
      if (next?.type === "hash") {
        const afterHash = peekAt(state, 2);
        if (afterHash?.type === "kw_else") {
          break;
        }
      }
    }

    const node = parseNode(state);
    if (node) {
      nodes.push(node);
    }
  }

  return nodes;
}

/**
 * Parse tokens into an AST Template.
 */
export function parse(tokens: Token[]): Template {
  const state = createState(tokens);
  const nodes = parseNodes(state);
  return template(nodes);
}
