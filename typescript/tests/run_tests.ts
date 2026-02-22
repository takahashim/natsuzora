/**
 * Natsuzora Shared Test Runner
 *
 * Runs the shared JSON test cases for Deno, Node.js, and Bun.
 */

import { render, parse } from "../src/mod.ts";
import {
  NatsuzoraError,
  LexerError,
  ParseError,
  ReservedWordError,
  RenderError,
  UndefinedVariableError,
  TypeError,
  NullValueError,
  EmptyStringError,
  ShadowingError,
} from "../src/mod.ts";

// Platform detection
const isDeno = typeof Deno !== "undefined";

interface TestCase {
  name: string;
  template: string;
  data: Record<string, unknown>;
  expected?: string;
  error?: string;
}

interface TestFile {
  description: string;
  tests: TestCase[];
}

// File reading abstraction
async function readJsonFile(path: string): Promise<TestFile> {
  if (isDeno) {
    const text = await Deno.readTextFile(path);
    return JSON.parse(text);
  } else {
    // Node.js / Bun
    const fs = await import("node:fs/promises");
    const text = await fs.readFile(path, "utf-8");
    return JSON.parse(text);
  }
}

// Get test files directory
function getTestsDir(): string {
  if (isDeno) {
    // Deno: use import.meta
    const url = new URL("../../tests", import.meta.url);
    return url.pathname;
  } else {
    // Node.js / Bun
    const path = require("node:path") as typeof import("node:path");
    return path.resolve(__dirname, "../../tests");
  }
}

// Map error type string to error class
function getExpectedErrorType(
  errorName: string
): new (...args: unknown[]) => Error {
  switch (errorName) {
    case "LexerError":
      return LexerError;
    case "ParseError":
      return ParseError;
    case "ReservedWordError":
      return ReservedWordError;
    case "RenderError":
      return RenderError;
    case "UndefinedVariable":
      return UndefinedVariableError;
    case "TypeError":
      return TypeError;
    case "NullValueError":
      return NullValueError;
    case "EmptyStringError":
      return EmptyStringError;
    case "ShadowingError":
      return ShadowingError;
    default:
      return NatsuzoraError;
  }
}

// Check if error matches expected type
function errorMatches(error: Error, expectedType: string): boolean {
  // SyntaxError matches both LexerError and ParseError (implementation detail)
  if (expectedType === "SyntaxError") {
    return error instanceof LexerError || error instanceof ParseError;
  }
  const ErrorClass = getExpectedErrorType(expectedType);
  return error instanceof ErrorClass;
}

// Run a single test
function runTest(test: TestCase): { passed: boolean; message?: string } {
  try {
    // First try to parse (may fail for parse errors)
    parse(test.template);

    // Then try to render
    const result = render(test.template, test.data);

    if (test.error) {
      return {
        passed: false,
        message: `Expected error '${test.error}', but got result: "${result}"`,
      };
    }

    if (result !== test.expected) {
      return {
        passed: false,
        message: `Expected "${test.expected}", but got "${result}"`,
      };
    }

    return { passed: true };
  } catch (e) {
    if (test.error) {
      if (e instanceof Error && errorMatches(e, test.error)) {
        return { passed: true };
      }
      return {
        passed: false,
        message: `Expected error '${test.error}', but got: ${e instanceof Error ? e.constructor.name : String(e)} - ${e}`,
      };
    }

    return {
      passed: false,
      message: `Unexpected error: ${e instanceof Error ? e.message : String(e)}`,
    };
  }
}

// Test files to run (exclude include.json for now as it requires file system setup)
const TEST_FILES = [
  "basic.json",
  "if_block.json",
  "each_block.json",
  "unsecure.json",
  "truthiness.json",
  "stringify.json",
  "errors.json",
];

async function main() {
  const testsDir = getTestsDir();
  let totalPassed = 0;
  let totalFailed = 0;
  const failures: Array<{ file: string; test: string; message: string }> = [];

  console.log("Running Natsuzora shared test cases...\n");

  for (const fileName of TEST_FILES) {
    const filePath = `${testsDir}/${fileName}`;
    let testFile: TestFile;

    try {
      testFile = await readJsonFile(filePath);
    } catch (e) {
      console.log(`  ❌ Failed to load ${fileName}: ${e}`);
      totalFailed++;
      continue;
    }

    console.log(`📁 ${fileName}: ${testFile.description}`);

    for (const test of testFile.tests) {
      const result = runTest(test);

      if (result.passed) {
        console.log(`  ✓ ${test.name}`);
        totalPassed++;
      } else {
        console.log(`  ✗ ${test.name}`);
        console.log(`    ${result.message}`);
        totalFailed++;
        failures.push({
          file: fileName,
          test: test.name,
          message: result.message || "Unknown error",
        });
      }
    }

    console.log("");
  }

  console.log("─".repeat(50));
  console.log(`Total: ${totalPassed} passed, ${totalFailed} failed`);

  if (failures.length > 0) {
    console.log("\nFailures:");
    for (const f of failures) {
      console.log(`  ${f.file} / ${f.test}`);
      console.log(`    ${f.message}`);
    }
    if (isDeno) {
      Deno.exit(1);
    } else {
      process.exit(1);
    }
  }
}

main().catch((e) => {
  console.error("Test runner failed:", e);
  if (isDeno) {
    Deno.exit(1);
  } else {
    process.exit(1);
  }
});
