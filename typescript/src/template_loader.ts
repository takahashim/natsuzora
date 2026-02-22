/**
 * Natsuzora Template Loader
 *
 * Loads and caches partial templates for include support.
 */

import { IncludeError } from "./errors.ts";
import { Template } from "./ast.ts";
import { tokenize } from "./lexer.ts";
import { parse } from "./parser.ts";
import { TemplateLoader as ITemplateLoader } from "./renderer.ts";
import { readFileSync, joinPath, normalizePath } from "./platform.ts";

interface LoaderState {
  includeRoot: string;
  cache: Map<string, Template>;
  includeStack: string[];
}

function createLoaderState(includeRoot: string): LoaderState {
  return {
    includeRoot: normalizePath(includeRoot),
    cache: new Map(),
    includeStack: [],
  };
}

function validateIncludeRoot(state: LoaderState): void {
  if (!state.includeRoot) {
    throw new IncludeError("include_root is not configured");
  }
}

function validateName(name: string): void {
  if (!name.startsWith("/")) {
    throw new IncludeError(`Include name must start with '/': ${name}`);
  }

  if (name.includes("..")) {
    throw new IncludeError(`Include name cannot contain '..': ${name}`);
  }

  if (name.includes("//")) {
    throw new IncludeError(`Include name cannot contain '//': ${name}`);
  }
}

function resolveIncludePath(state: LoaderState, name: string): string {
  const segments = name.split("/").filter((s) => s !== "");

  if (segments.length === 0) {
    throw new IncludeError(`Invalid include name: ${name}`);
  }

  // Prepend underscore to the last segment (partial naming convention)
  segments[segments.length - 1] = "_" + segments[segments.length - 1];

  return joinPath(state.includeRoot, ...segments) + ".ntzr";
}

function validatePathSecurity(state: LoaderState, path: string): void {
  const normalized = normalizePath(path);
  const rootNormalized = normalizePath(state.includeRoot);

  if (!normalized.startsWith(rootNormalized)) {
    throw new IncludeError(`Path traversal detected: ${path}`);
  }
}

function loadAndParse(state: LoaderState, name: string): Template {
  const path = resolveIncludePath(state, name);
  validatePathSecurity(state, path);

  let source: string;
  try {
    source = readFileSync(path);
  } catch {
    throw new IncludeError(`Include file not found: ${name} (${path})`);
  }

  const tokens = tokenize(source);
  return parse(tokens);
}

function load(state: LoaderState, name: string): Template {
  validateIncludeRoot(state);
  validateName(name);

  if (state.includeStack.includes(name)) {
    throw new IncludeError(`Circular include detected: ${name}`);
  }

  let cached = state.cache.get(name);
  if (!cached) {
    cached = loadAndParse(state, name);
    state.cache.set(name, cached);
  }

  return cached;
}

function withInclude<T>(state: LoaderState, name: string, fn: () => T): T {
  state.includeStack.push(name);
  try {
    return fn();
  } finally {
    state.includeStack.pop();
  }
}

/**
 * Create a template loader.
 */
export function createTemplateLoader(includeRoot: string): ITemplateLoader {
  const state = createLoaderState(includeRoot);

  return {
    load: (name: string) => load(state, name),
    withInclude: <T>(name: string, fn: () => T) => withInclude(state, name, fn),
  };
}
