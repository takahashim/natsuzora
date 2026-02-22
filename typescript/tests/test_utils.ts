/**
 * Test Utilities - Platform-agnostic test framework
 *
 * Deno.test style API with t.step support. No external dependencies.
 * Works on Deno, Node.js, and Bun.
 */

// Platform detection
declare const Deno: { exit(code: number): never } | undefined;

function exit(code: number): never {
  if (typeof Deno !== "undefined") {
    Deno.exit(code);
  }
  process.exit(code);
}

// =============================================================================
// Assertion functions
// =============================================================================

export class AssertionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AssertionError";
  }
}

/**
 * Deep equality check
 */
function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;

  if (a === null || b === null) return a === b;
  if (typeof a !== typeof b) return false;

  if (typeof a === "object") {
    if (Array.isArray(a) && Array.isArray(b)) {
      if (a.length !== b.length) return false;
      for (let i = 0; i < a.length; i++) {
        if (!deepEqual(a[i], b[i])) return false;
      }
      return true;
    }

    if (Array.isArray(a) || Array.isArray(b)) return false;

    const aObj = a as Record<string, unknown>;
    const bObj = b as Record<string, unknown>;
    const aKeys = Object.keys(aObj);
    const bKeys = Object.keys(bObj);

    if (aKeys.length !== bKeys.length) return false;

    for (const key of aKeys) {
      if (!Object.prototype.hasOwnProperty.call(bObj, key)) return false;
      if (!deepEqual(aObj[key], bObj[key])) return false;
    }

    return true;
  }

  return false;
}

/**
 * Format value for error messages
 */
function format(value: unknown): string {
  if (value === null) return "null";
  if (value === undefined) return "undefined";
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "object") {
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  return String(value);
}

/**
 * Assert two values are deeply equal
 */
export function assertEquals<T>(actual: T, expected: T, msg?: string): void {
  if (!deepEqual(actual, expected)) {
    const message = msg
      ? msg
      : `Values are not equal:\n  actual:   ${format(actual)}\n  expected: ${format(expected)}`;
    throw new AssertionError(message);
  }
}

/**
 * Assert a value is true
 */
export function assertTrue(actual: unknown, msg?: string): void {
  if (actual !== true) {
    throw new AssertionError(msg || `Expected true, got ${format(actual)}`);
  }
}

/**
 * Assert a value is false
 */
export function assertFalse(actual: unknown, msg?: string): void {
  if (actual !== false) {
    throw new AssertionError(msg || `Expected false, got ${format(actual)}`);
  }
}

/**
 * Assert a function throws an error
 */
export function assertThrows(
  fn: () => unknown,
  errorClass?: new (...args: unknown[]) => Error,
  msgIncludes?: string
): Error {
  let threw = false;
  let error: Error | null = null;

  try {
    fn();
  } catch (e) {
    threw = true;
    error = e instanceof Error ? e : new Error(String(e));
  }

  if (!threw) {
    throw new AssertionError("Expected function to throw, but it did not");
  }

  if (errorClass && !(error instanceof errorClass)) {
    throw new AssertionError(
      `Expected error to be instance of ${errorClass.name}, got ${error?.constructor.name}`
    );
  }

  if (msgIncludes && error && !error.message.includes(msgIncludes)) {
    throw new AssertionError(
      `Expected error message to include "${msgIncludes}", got "${error.message}"`
    );
  }

  return error!;
}

/**
 * Assert a value is not null or undefined
 */
export function assertExists<T>(actual: T, msg?: string): asserts actual is NonNullable<T> {
  if (actual === null || actual === undefined) {
    throw new AssertionError(msg || `Expected value to exist, got ${format(actual)}`);
  }
}

// =============================================================================
// Test runner (Deno.test style with t.step support)
// =============================================================================

interface StepResult {
  name: string;
  passed: boolean;
  error?: Error;
}

export interface TestContext {
  step(name: string, fn: () => void | Promise<void>): Promise<boolean>;
}

interface TestCase {
  name: string;
  fn: (t: TestContext) => void | Promise<void>;
}

const tests: TestCase[] = [];

/**
 * Register a test case (Deno.test style)
 */
export function test(name: string, fn: (t: TestContext) => void | Promise<void>): void {
  tests.push({ name, fn });
}

/**
 * Run all registered tests
 */
export async function runTests(): Promise<void> {
  let totalPassed = 0;
  let totalFailed = 0;
  const failures: Array<{ name: string; error: Error }> = [];

  console.log("Running tests...\n");

  for (const t of tests) {
    const steps: StepResult[] = [];
    let hasSteps = false;

    const ctx: TestContext = {
      async step(name: string, fn: () => void | Promise<void>): Promise<boolean> {
        hasSteps = true;
        try {
          await fn();
          steps.push({ name, passed: true });
          return true;
        } catch (e) {
          const error = e instanceof Error ? e : new Error(String(e));
          steps.push({ name, passed: false, error });
          return false;
        }
      },
    };

    try {
      await t.fn(ctx);

      if (hasSteps) {
        // Test with steps
        const stepsPassed = steps.filter((s) => s.passed).length;
        const stepsFailed = steps.filter((s) => !s.passed).length;

        if (stepsFailed === 0) {
          console.log(`✓ ${t.name} (${stepsPassed} steps)`);
        } else {
          console.log(`✗ ${t.name} (${stepsPassed}/${steps.length} steps passed)`);
        }

        for (const step of steps) {
          if (step.passed) {
            console.log(`  ✓ ${step.name}`);
            totalPassed++;
          } else {
            console.log(`  ✗ ${step.name}`);
            console.log(`    ${step.error?.message}`);
            totalFailed++;
            failures.push({ name: `${t.name} > ${step.name}`, error: step.error! });
          }
        }
      } else {
        // Simple test without steps
        console.log(`✓ ${t.name}`);
        totalPassed++;
      }
    } catch (e) {
      // Top-level test failure (not in a step)
      const error = e instanceof Error ? e : new Error(String(e));
      console.log(`✗ ${t.name}`);
      console.log(`  ${error.message}`);
      totalFailed++;
      failures.push({ name: t.name, error });
    }
  }

  console.log("\n" + "─".repeat(50));
  console.log(`Total: ${totalPassed} passed, ${totalFailed} failed`);

  if (failures.length > 0) {
    console.log("\nFailures:");
    for (const f of failures) {
      console.log(`  ${f.name}`);
      console.log(`    ${f.error.message}`);
    }
    exit(1);
  }
}
