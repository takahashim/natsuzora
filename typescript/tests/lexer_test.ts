/**
 * Lexer unit tests
 */

import {
  test,
  assertEquals,
  assertTrue,
  assertThrows,
  runTests,
} from "./test_utils.ts";
import { tokenize } from "../src/lexer.ts";
import { LexerError } from "../src/errors.ts";
import { Token } from "../src/token.ts";

// Helper to extract token types (ignoring eof and whitespace)
function types(tokens: Token[]): string[] {
  return tokens
    .filter((t) => t.type !== "eof" && t.type !== "whitespace")
    .map((t) => t.type);
}

// Helper to extract token values (ignoring eof and whitespace)
function values(tokens: Token[]): (string | null)[] {
  return tokens
    .filter((t) => t.type !== "eof" && t.type !== "whitespace")
    .map((t) => t.value);
}

// =============================================================================
// Basic tokenization
// =============================================================================

test("Lexer - basic text", async (t) => {
  await t.step("empty string", () => {
    const tokens = tokenize("");
    assertEquals(types(tokens), []);
  });

  await t.step("plain text only", () => {
    const tokens = tokenize("Hello, world!");
    assertEquals(types(tokens), ["text"]);
    assertEquals(values(tokens), ["Hello, world!"]);
  });

  await t.step("text with newlines", () => {
    const tokens = tokenize("line1\nline2\nline3");
    assertEquals(types(tokens), ["text"]);
    assertEquals(values(tokens), ["line1\nline2\nline3"]);
  });
});

test("Lexer - variable tags", async (t) => {
  await t.step("simple variable", () => {
    const tokens = tokenize("{[ name ]}");
    assertEquals(types(tokens), ["open", "ident", "close"]);
    assertEquals(values(tokens)[1], "name");
  });

  await t.step("variable without spaces", () => {
    const tokens = tokenize("{[name]}");
    assertEquals(types(tokens), ["open", "ident", "close"]);
  });

  await t.step("variable with extra spaces", () => {
    const tokens = tokenize("{[   name   ]}");
    assertEquals(types(tokens), ["open", "ident", "close"]);
  });

  await t.step("dotted path", () => {
    const tokens = tokenize("{[ user.name ]}");
    assertEquals(types(tokens), ["open", "ident", "dot", "ident", "close"]);
  });

  await t.step("deep path", () => {
    const tokens = tokenize("{[ a.b.c.d ]}");
    assertEquals(types(tokens), ["open", "ident", "dot", "ident", "dot", "ident", "dot", "ident", "close"]);
  });

  await t.step("text before and after variable", () => {
    const tokens = tokenize("Hello, {[ name ]}!");
    assertEquals(types(tokens), ["text", "open", "ident", "close", "text"]);
    assertEquals(values(tokens)[0], "Hello, ");
    assertEquals(values(tokens)[4], "!");
  });
});

// =============================================================================
// Keywords
// =============================================================================

test("Lexer - keywords", async (t) => {
  await t.step("if keyword", () => {
    const tokens = tokenize("{[#if]}");
    assertEquals(types(tokens), ["open", "hash", "kw_if", "close"]);
  });

  await t.step("unless keyword", () => {
    const tokens = tokenize("{[#unless]}");
    assertEquals(types(tokens), ["open", "hash", "kw_unless", "close"]);
  });

  await t.step("else keyword", () => {
    const tokens = tokenize("{[#else]}");
    assertEquals(types(tokens), ["open", "hash", "kw_else", "close"]);
  });

  await t.step("each keyword", () => {
    const tokens = tokenize("{[#each]}");
    assertEquals(types(tokens), ["open", "hash", "kw_each", "close"]);
  });

  await t.step("as keyword", () => {
    const tokens = tokenize("{[#each items as item]}");
    const tokenTypes = types(tokens);
    assertTrue(tokenTypes.includes("kw_as"));
  });
});

// =============================================================================
// Block syntax
// =============================================================================

test("Lexer - block syntax", async (t) => {
  await t.step("if block open", () => {
    const tokens = tokenize("{[#if visible]}");
    assertEquals(types(tokens), ["open", "hash", "kw_if", "ident", "close"]);
  });

  await t.step("block close with slash", () => {
    const tokens = tokenize("{[/if]}");
    assertEquals(types(tokens), ["open", "slash", "kw_if", "close"]);
  });

  await t.step("each with as and comma", () => {
    const tokens = tokenize("{[#each items as item, index]}");
    assertEquals(types(tokens), [
      "open", "hash", "kw_each", "ident", "kw_as", "ident", "comma", "ident", "close"
    ]);
  });
});

// =============================================================================
// Comments
// =============================================================================

test("Lexer - comments", async (t) => {
  await t.step("simple comment", () => {
    const tokens = tokenize("{[% this is a comment ]}");
    assertEquals(types(tokens), []);
  });

  await t.step("comment with surrounding text", () => {
    const tokens = tokenize("before{[% comment ]}after");
    assertEquals(types(tokens), ["text", "text"]);
    assertEquals(values(tokens), ["before", "after"]);
  });

  await t.step("multiline comment", () => {
    const tokens = tokenize("{[% line1\nline2\nline3 ]}");
    assertEquals(types(tokens), []);
  });

  await t.step("unclosed comment throws", () => {
    assertThrows(() => tokenize("{[% unclosed"), LexerError, "Unclosed comment");
  });
});

// =============================================================================
// Whitespace control
// =============================================================================

test("Lexer - whitespace control", async (t) => {
  await t.step("strip before with {[-", () => {
    const tokens = tokenize("line1\n  {[- name ]}");
    const textTokens = tokens.filter((tok) => tok.type === "text");
    assertEquals(textTokens[0].value, "line1\n");
  });

  await t.step("strip after with -]}", () => {
    const tokens = tokenize("{[ name -]}\n  next");
    const textTokens = tokens.filter((tok) => tok.type === "text");
    assertEquals(textTokens[0].value, "next");
  });

  await t.step("strip both sides", () => {
    const tokens = tokenize("before\n  {[- name -]}\n  after");
    const textTokens = tokens.filter((tok) => tok.type === "text");
    assertEquals(textTokens[0].value, "before\n");
    assertEquals(textTokens[1].value, "after");
  });
});

// =============================================================================
// Delimiter escape
// =============================================================================

test("Lexer - delimiter escape", async (t) => {
  await t.step("{[{]} produces literal {[", () => {
    const tokens = tokenize("{[{]}");
    assertEquals(types(tokens), ["text"]);
    assertEquals(values(tokens), ["{["]);
  });

  await t.step("escape with surrounding text", () => {
    const tokens = tokenize("Use {[{]} to start a tag");
    const textTokens = tokens.filter((tok) => tok.type === "text");
    assertEquals(textTokens.length, 3);
    assertEquals(textTokens[0].value, "Use ");
    assertEquals(textTokens[1].value, "{[");
    assertEquals(textTokens[2].value, " to start a tag");
  });

  await t.step("multiple escapes", () => {
    const tokens = tokenize("{[{]}{[{]}");
    assertEquals(types(tokens), ["text", "text"]);
    assertEquals(values(tokens), ["{[", "{["]);
  });
});

// =============================================================================
// Bang commands
// =============================================================================

test("Lexer - unsecure", async (t) => {
  await t.step("!unsecure token", () => {
    const tokens = tokenize("{[!unsecure html ]}");
    assertEquals(types(tokens), ["open", "bang_unsecure", "ident", "close"]);
  });

  await t.step("!unsecure with path", () => {
    const tokens = tokenize("{[!unsecure user.bio ]}");
    assertEquals(types(tokens), ["open", "bang_unsecure", "ident", "dot", "ident", "close"]);
  });
});

test("Lexer - include", async (t) => {
  await t.step("!include token", () => {
    const tokens = tokenize("{[!include /header ]}");
    assertEquals(types(tokens), ["open", "bang_include", "ident", "close"]);
    assertEquals(values(tokens)[2], "/header");
  });

  await t.step("!include with nested path", () => {
    const tokens = tokenize("{[!include /shared/footer ]}");
    assertEquals(types(tokens), ["open", "bang_include", "ident", "close"]);
    assertEquals(values(tokens)[2], "/shared/footer");
  });

  await t.step("!include with arguments", () => {
    const tokens = tokenize("{[!include /card title=item.name ]}");
    assertEquals(types(tokens), [
      "open", "bang_include", "ident", "ident", "equal", "ident", "dot", "ident", "close"
    ]);
  });

  await t.step("invalid bang command throws", () => {
    assertThrows(() => tokenize("{[!invalid ]}"), LexerError, "Expected 'unsecure' or 'include'");
  });
});

// =============================================================================
// Error cases
// =============================================================================

test("Lexer - error cases", async (t) => {
  await t.step("identifier starting with underscore throws", () => {
    assertThrows(() => tokenize("{[ _private ]}"), LexerError);
  });

  await t.step("whitespace before # throws", () => {
    assertThrows(() => tokenize("{[ #if x ]}"), LexerError, "Whitespace not allowed");
  });

  await t.step("whitespace before / throws", () => {
    assertThrows(() => tokenize("{[ /if ]}"), LexerError, "Whitespace not allowed");
  });

  await t.step("whitespace before ! throws", () => {
    assertThrows(() => tokenize("{[ !unsecure x ]}"), LexerError, "Whitespace not allowed");
  });

  await t.step("unexpected character throws", () => {
    assertThrows(() => tokenize("{[ @ ]}"), LexerError, "Unexpected character");
  });

  await t.step("include path starting with underscore throws", () => {
    assertThrows(() => tokenize("{[!include /_private ]}"), LexerError);
  });

  await t.step("include path starting with digit throws", () => {
    assertThrows(() => tokenize("{[!include /123 ]}"), LexerError);
  });
});

// =============================================================================
// Line/column tracking
// =============================================================================

test("Lexer - position tracking", async (t) => {
  await t.step("first token at line 1, column 1", () => {
    const tokens = tokenize("hello");
    assertEquals(tokens[0].line, 1);
    assertEquals(tokens[0].column, 1);
  });

  await t.step("tracks line numbers", () => {
    const tokens = tokenize("line1\nline2\n{[ x ]}");
    const openToken = tokens.find((tok) => tok.type === "open")!;
    assertEquals(openToken.line, 3);
  });

  await t.step("error includes position", () => {
    try {
      tokenize("ok\n{[ _bad ]}");
    } catch (e) {
      if (e instanceof LexerError) {
        assertEquals(e.line, 2);
      }
    }
  });
});

// Run tests
runTests();
