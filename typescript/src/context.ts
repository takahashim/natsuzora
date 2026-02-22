/**
 * Natsuzora Context
 *
 * Scope stack management and name resolution.
 * Functional implementation without classes.
 */

import { UndefinedVariableError, ShadowingError, TypeError } from "./errors.ts";
import { Path } from "./ast.ts";
import { normalizeRootData, normalizeBindings } from "./value.ts";

export interface Context {
  root: Record<string, unknown>;
  localStack: Array<Record<string, unknown>>;
}

/**
 * Create a new context from root data.
 */
export function createContext(rootData: Record<string, unknown>): Context {
  return {
    root: normalizeRootData(rootData),
    localStack: [],
  };
}

/**
 * Resolve a path to its value in the context.
 * Looks up in local scopes first, then root.
 */
export function resolve(ctx: Context, path: Path): unknown {
  const name = path.segments[0];
  let value = resolveName(ctx, name);

  for (let i = 1; i < path.segments.length; i++) {
    value = accessProperty(value, path.segments[i]);
  }

  return value;
}

function resolveName(ctx: Context, name: string): unknown {
  // Search from top of stack (most recent scope) to bottom
  for (let i = ctx.localStack.length - 1; i >= 0; i--) {
    const scope = ctx.localStack[i];
    if (Object.prototype.hasOwnProperty.call(scope, name)) {
      return scope[name];
    }
  }

  // Check root
  if (Object.prototype.hasOwnProperty.call(ctx.root, name)) {
    return ctx.root[name];
  }

  throw new UndefinedVariableError(name);
}

function accessProperty(value: unknown, key: string): unknown {
  if (value === null || value === undefined) {
    throw new TypeError(`Cannot access property '${key}' on null`);
  }

  if (typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`Cannot access property '${key}' on non-object`);
  }

  const obj = value as Record<string, unknown>;

  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new UndefinedVariableError(key);
  }

  return obj[key];
}

function nameExists(ctx: Context, name: string): boolean {
  // Check local scopes
  for (const scope of ctx.localStack) {
    if (Object.prototype.hasOwnProperty.call(scope, name)) {
      return true;
    }
  }

  // Check root
  return Object.prototype.hasOwnProperty.call(ctx.root, name);
}

function validateNoShadowing(ctx: Context, bindings: Record<string, unknown>): void {
  for (const name of Object.keys(bindings)) {
    if (nameExists(ctx, name)) {
      throw new ShadowingError(name);
    }
  }
}

/**
 * Push a new scope onto the stack.
 * Validates that bindings don't shadow existing names.
 */
export function pushScope(ctx: Context, bindings: Record<string, unknown>): void {
  validateNoShadowing(ctx, bindings);
  ctx.localStack.push(normalizeBindings(bindings));
}

/**
 * Push an include scope onto the stack.
 * Skip shadowing validation (include args are allowed to shadow).
 */
export function pushIncludeScope(ctx: Context, bindings: Record<string, unknown>): void {
  ctx.localStack.push(normalizeBindings(bindings));
}

/**
 * Pop the top scope from the stack.
 */
export function popScope(ctx: Context): void {
  ctx.localStack.pop();
}

/**
 * Execute a function within a new scope.
 */
export function withScope<T>(
  ctx: Context,
  bindings: Record<string, unknown>,
  fn: () => T,
  includeScope = false
): T {
  if (includeScope) {
    pushIncludeScope(ctx, bindings);
  } else {
    pushScope(ctx, bindings);
  }

  try {
    return fn();
  } finally {
    popScope(ctx);
  }
}
