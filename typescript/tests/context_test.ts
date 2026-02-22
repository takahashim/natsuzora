/**
 * Context unit tests
 */

import {
  test,
  assertEquals,
  assertTrue,
  assertThrows,
  runTests,
} from "./test_utils.ts";
import { createContext, resolve, pushScope, popScope } from "../src/context.ts";
import { UndefinedVariableError, ShadowingError, TypeError } from "../src/errors.ts";
import { createPath } from "../src/ast.ts";

// Helper to create a path from segments
function path(...segments: string[]) {
  return createPath(segments);
}

// =============================================================================
// Basic resolution
// =============================================================================

test("Context - basic resolution", async (t) => {
  await t.step("resolve simple variable", () => {
    const ctx = createContext({ name: "Alice" });
    const result = resolve(ctx, path("name"));
    assertEquals(result, "Alice");
  });

  await t.step("resolve integer value", () => {
    const ctx = createContext({ count: 42 });
    const result = resolve(ctx, path("count"));
    assertEquals(result, 42);
  });

  await t.step("resolve boolean true", () => {
    const ctx = createContext({ flag: true });
    const result = resolve(ctx, path("flag"));
    assertEquals(result, true);
  });

  await t.step("resolve boolean false", () => {
    const ctx = createContext({ flag: false });
    const result = resolve(ctx, path("flag"));
    assertEquals(result, false);
  });

  await t.step("resolve null value", () => {
    const ctx = createContext({ value: null });
    const result = resolve(ctx, path("value"));
    assertEquals(result, null);
  });

  await t.step("resolve array", () => {
    const ctx = createContext({ items: [1, 2, 3] });
    const result = resolve(ctx, path("items"));
    assertEquals(result, [1, 2, 3]);
  });

  await t.step("resolve object", () => {
    const ctx = createContext({ user: { name: "Bob" } });
    const result = resolve(ctx, path("user"));
    assertEquals(result, { name: "Bob" });
  });
});

// =============================================================================
// Nested path resolution
// =============================================================================

test("Context - nested path resolution", async (t) => {
  await t.step("resolve dotted path", () => {
    const ctx = createContext({ user: { name: "Alice" } });
    const result = resolve(ctx, path("user", "name"));
    assertEquals(result, "Alice");
  });

  await t.step("resolve deep path", () => {
    const ctx = createContext({
      a: { b: { c: { d: "value" } } },
    });
    const result = resolve(ctx, path("a", "b", "c", "d"));
    assertEquals(result, "value");
  });

  await t.step("resolve path with null intermediate throws TypeError", () => {
    const ctx = createContext({ user: null });
    assertThrows(
      () => resolve(ctx, path("user", "name")),
      TypeError,
      "null"
    );
  });

  await t.step("resolve path with missing property throws UndefinedVariableError", () => {
    const ctx = createContext({ user: {} });
    assertThrows(
      () => resolve(ctx, path("user", "name")),
      UndefinedVariableError,
      "name"
    );
  });
});

// =============================================================================
// Scope management
// =============================================================================

test("Context - scope management", async (t) => {
  await t.step("push and pop scope", () => {
    const ctx = createContext({ name: "root" });
    pushScope(ctx, { item: "local" });

    const itemResult = resolve(ctx, path("item"));
    assertEquals(itemResult, "local");

    popScope(ctx);

    // After pop, item should not be accessible
    assertThrows(() => resolve(ctx, path("item")), UndefinedVariableError);
  });

  await t.step("multiple nested scopes", () => {
    const ctx = createContext({ level: 0 });
    pushScope(ctx, { level1: 1 });
    pushScope(ctx, { level2: 2 });

    assertEquals(resolve(ctx, path("level")), 0);
    assertEquals(resolve(ctx, path("level1")), 1);
    assertEquals(resolve(ctx, path("level2")), 2);

    popScope(ctx);
    assertEquals(resolve(ctx, path("level1")), 1);
    assertThrows(() => resolve(ctx, path("level2")), UndefinedVariableError);

    popScope(ctx);
    assertThrows(() => resolve(ctx, path("level1")), UndefinedVariableError);
  });

  await t.step("root data always accessible", () => {
    const ctx = createContext({ root: "value" });
    pushScope(ctx, { local: "data" });

    assertEquals(resolve(ctx, path("root")), "value");
    assertEquals(resolve(ctx, path("local")), "data");

    popScope(ctx);
    assertEquals(resolve(ctx, path("root")), "value");
  });
});

// =============================================================================
// Shadowing detection
// =============================================================================

test("Context - shadowing detection", async (t) => {
  await t.step("shadowing existing variable throws", () => {
    const ctx = createContext({ name: "Alice" });
    assertThrows(
      () => pushScope(ctx, { name: "Bob" }),
      ShadowingError,
      "name"
    );
  });

  await t.step("shadowing nested scope variable throws", () => {
    const ctx = createContext({});
    pushScope(ctx, { item: "first" });
    assertThrows(
      () => pushScope(ctx, { item: "second" }),
      ShadowingError,
      "item"
    );
    popScope(ctx);
  });

  await t.step("shadowing root variable from nested scope throws", () => {
    const ctx = createContext({ count: 1 });
    pushScope(ctx, { local: "a" });
    assertThrows(
      () => pushScope(ctx, { count: 2 }),
      ShadowingError,
      "count"
    );
    popScope(ctx);
  });

  await t.step("reusing name after pop is allowed", () => {
    const ctx = createContext({});
    pushScope(ctx, { item: "first" });
    popScope(ctx);

    // Should not throw - name is no longer in scope
    pushScope(ctx, { item: "second" });
    assertEquals(resolve(ctx, path("item")), "second");
    popScope(ctx);
  });

  await t.step("different names in same scope is allowed", () => {
    const ctx = createContext({ a: 1 });
    pushScope(ctx, { b: 2 });
    pushScope(ctx, { c: 3 });

    assertEquals(resolve(ctx, path("a")), 1);
    assertEquals(resolve(ctx, path("b")), 2);
    assertEquals(resolve(ctx, path("c")), 3);

    popScope(ctx);
    popScope(ctx);
  });
});

// =============================================================================
// Error cases
// =============================================================================

test("Context - error cases", async (t) => {
  await t.step("undefined variable throws", () => {
    const ctx = createContext({});
    assertThrows(
      () => resolve(ctx, path("notExist")),
      UndefinedVariableError,
      "notExist"
    );
  });

  await t.step("undefined first segment throws", () => {
    const ctx = createContext({ user: { name: "Alice" } });
    assertThrows(
      () => resolve(ctx, path("other", "name")),
      UndefinedVariableError,
      "other"
    );
  });

  await t.step("error contains variable name", () => {
    const ctx = createContext({});
    try {
      resolve(ctx, path("missing"));
    } catch (e) {
      if (e instanceof UndefinedVariableError) {
        assertEquals(e.variableName, "missing");
        assertTrue(e.message.includes("missing"));
      }
    }
  });

  await t.step("shadowing error contains variable name", () => {
    const ctx = createContext({ x: 1 });
    try {
      pushScope(ctx, { x: 2 });
    } catch (e) {
      if (e instanceof ShadowingError) {
        assertEquals(e.variableName, "x");
        assertTrue(e.message.includes("x"));
      }
    }
  });
});

// =============================================================================
// Data normalization
// =============================================================================

test("Context - data normalization", async (t) => {
  await t.step("whole number float is normalized to integer", () => {
    const ctx = createContext({ value: 42.0 });
    const result = resolve(ctx, path("value"));
    assertEquals(result, 42);
  });

  await t.step("nested whole number float is normalized", () => {
    const ctx = createContext({ data: { count: 100.0 } });
    const result = resolve(ctx, path("data", "count"));
    assertEquals(result, 100);
  });

  await t.step("array with whole number floats is normalized", () => {
    const ctx = createContext({ nums: [1.0, 2.0, 3.0] });
    const result = resolve(ctx, path("nums"));
    assertEquals(result, [1, 2, 3]);
  });
});

// Run tests
runTests();
