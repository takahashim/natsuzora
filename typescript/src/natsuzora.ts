/**
 * Natsuzora - A Minimal Template Language
 *
 * Public API for parsing and rendering templates.
 */

import { Template } from "./ast.ts";
import { tokenize } from "./lexer.ts";
import { parse as parseTokens } from "./parser.ts";
import { render as renderAst, TemplateLoader } from "./renderer.ts";
import { createTemplateLoader } from "./template_loader.ts";
import { readFileSync } from "./platform.ts";

export interface RenderOptions {
  /**
   * Root directory for include paths.
   * Required when using includes.
   */
  includeRoot?: string;
}

export interface ParsedTemplate {
  ast: Template;
  templateLoader: TemplateLoader | null;
}

/**
 * Parse a template source string into an AST.
 */
export function parse(source: string): Template {
  const tokens = tokenize(source);
  return parseTokens(tokens);
}

/**
 * Parse a template with include support.
 */
export function parseWithIncludes(source: string, includeRoot: string): ParsedTemplate {
  const ast = parse(source);
  const templateLoader = createTemplateLoader(includeRoot);
  return { ast, templateLoader };
}

/**
 * Render a parsed template with the given data.
 */
export function renderTemplate(
  template: ParsedTemplate,
  data: Record<string, unknown>
): string {
  return renderAst(template.ast, data, template.templateLoader);
}

/**
 * Render a template source string with the given data.
 *
 * This is the main entry point for rendering templates.
 */
export function render(
  source: string,
  data: Record<string, unknown>,
  options: RenderOptions = {}
): string {
  const ast = parse(source);

  let templateLoader: TemplateLoader | null = null;
  if (options.includeRoot) {
    templateLoader = createTemplateLoader(options.includeRoot);
  }

  return renderAst(ast, data, templateLoader);
}

/**
 * Render a template file with the given data.
 */
export function renderFile(
  filePath: string,
  data: Record<string, unknown>,
  options: RenderOptions = {}
): string {
  const source = readFileSync(filePath);
  return render(source, data, options);
}

// Re-export types and utilities
export type { Template } from "./ast.ts";
export type { TemplateLoader } from "./renderer.ts";
export { createTemplateLoader } from "./template_loader.ts";

// Re-export error types
export {
  NatsuzoraError,
  LexerError,
  ParseError,
  ReservedWordError,
  RenderError,
  UndefinedVariableError,
  TypeError,
  IncludeError,
  ShadowingError,
} from "./errors.ts";
