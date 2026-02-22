/**
 * Natsuzora - A Minimal Template Language
 *
 * Module entry point.
 */

// Main API
export {
  parse,
  parseWithIncludes,
  render,
  renderFile,
  renderTemplate,
  createTemplateLoader,
} from "./natsuzora.ts";

// Types
export type {
  Template,
  ParsedTemplate,
  RenderOptions,
  TemplateLoader,
} from "./natsuzora.ts";

// Error types
export {
  NatsuzoraError,
  LexerError,
  ParseError,
  ReservedWordError,
  RenderError,
  UndefinedVariableError,
  TypeError,
  NullValueError,
  EmptyStringError,
  IncludeError,
  ShadowingError,
} from "./errors.ts";

// AST types (for advanced usage)
export type {
  Node,
  TextNode,
  VariableNode,
  IfBlockNode,
  UnlessBlockNode,
  EachBlockNode,
  UnsecureOutputNode,
  IncludeNode,
  IncludeArg,
  Path,
} from "./ast.ts";

// Low-level APIs (for advanced usage)
export { tokenize } from "./lexer.ts";
export { parse as parseTokens } from "./parser.ts";
export { render as renderAst } from "./renderer.ts";
export type { Token, TokenType } from "./token.ts";

// Platform utilities
export {
  readFileSync,
  fileExists,
  joinPath,
  normalizePath,
  isAbsolutePath,
  detectRuntime,
} from "./platform.ts";
export type { Runtime } from "./platform.ts";
