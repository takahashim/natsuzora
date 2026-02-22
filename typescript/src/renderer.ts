/**
 * Natsuzora Renderer
 *
 * Renders an AST to a string.
 * Functional implementation without classes.
 */

import { RenderError, IncludeError } from "./errors.ts";
import {
  Node,
  Template,
  TextNode,
  VariableNode,
  IfBlockNode,
  UnlessBlockNode,
  EachBlockNode,
  UnsecureOutputNode,
  IncludeNode,
} from "./ast.ts";
import { Context, createContext, resolve, withScope } from "./context.ts";
import { isTruthy, stringify, stringifyNullable, stringifyRequired, ensureArray } from "./value.ts";
import { escapeHtml } from "./html_escape.ts";

/**
 * Template loader interface for include support.
 */
export interface TemplateLoader {
  load(name: string): Template;
  withInclude<T>(name: string, fn: () => T): T;
}

interface RenderState {
  context: Context;
  templateLoader: TemplateLoader | null;
}

function createRenderState(
  data: Record<string, unknown>,
  templateLoader: TemplateLoader | null
): RenderState {
  return {
    context: createContext(data),
    templateLoader,
  };
}

function renderNodes(state: RenderState, nodes: Node[]): string {
  return nodes.map((node) => renderNode(state, node)).join("");
}

function renderNode(state: RenderState, node: Node): string {
  switch (node.type) {
    case "text":
      return renderText(node);
    case "variable":
      return renderVariable(state, node);
    case "if_block":
      return renderIf(state, node);
    case "unless_block":
      return renderUnless(state, node);
    case "each_block":
      return renderEach(state, node);
    case "unsecure_output":
      return renderUnsecureOutput(state, node);
    case "include":
      return renderInclude(state, node);
    default:
      throw new RenderError(`Unknown node type: ${(node as Node).type}`);
  }
}

function renderText(node: TextNode): string {
  return node.value;
}

function stringifyWithModifier(value: unknown, modifier: VariableNode["modifier"]): string {
  switch (modifier) {
    case "nullable":
      return stringifyNullable(value);
    case "required":
      return stringifyRequired(value);
    default:
      return stringify(value);
  }
}

function renderVariable(state: RenderState, node: VariableNode): string {
  const value = resolve(state.context, node.path);
  const str = stringifyWithModifier(value, node.modifier);
  return escapeHtml(str);
}

function renderIf(state: RenderState, node: IfBlockNode): string {
  const value = resolve(state.context, node.condition);

  if (isTruthy(value)) {
    return renderNodes(state, node.thenBranch);
  } else if (node.elseBranch) {
    return renderNodes(state, node.elseBranch);
  }

  return "";
}

function renderUnless(state: RenderState, node: UnlessBlockNode): string {
  const value = resolve(state.context, node.condition);

  if (isTruthy(value)) {
    return "";
  }

  return renderNodes(state, node.body);
}

function renderEach(state: RenderState, node: EachBlockNode): string {
  const collection = resolve(state.context, node.collection);
  const items = ensureArray(collection);

  const results: string[] = [];

  for (const item of items) {
    const bindings: Record<string, unknown> = {
      [node.itemName]: item,
    };

    const rendered = withScope(state.context, bindings, () => {
      return renderNodes(state, node.body);
    });

    results.push(rendered);
  }

  return results.join("");
}

function renderUnsecureOutput(state: RenderState, node: UnsecureOutputNode): string {
  const value = resolve(state.context, node.path);
  return stringify(value); // No HTML escaping
}

function renderInclude(state: RenderState, node: IncludeNode): string {
  if (!state.templateLoader) {
    throw new IncludeError("Template loader not configured for include");
  }

  const partialAst = state.templateLoader.load(node.name);

  const bindings: Record<string, unknown> = {};
  for (const arg of node.args) {
    bindings[arg.key] = resolve(state.context, arg.value);
  }

  return state.templateLoader.withInclude(node.name, () => {
    return withScope(
      state.context,
      bindings,
      () => renderNodes(state, partialAst.nodes),
      true // include scope (skip shadowing check)
    );
  });
}

/**
 * Render an AST template with the given data.
 */
export function render(
  ast: Template,
  data: Record<string, unknown>,
  templateLoader: TemplateLoader | null = null
): string {
  const state = createRenderState(data, templateLoader);
  return renderNodes(state, ast.nodes);
}
