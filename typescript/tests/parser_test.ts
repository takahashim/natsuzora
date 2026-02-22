/**
 * Parser unit tests
 */

import {
  test,
  assertEquals,
  assertTrue,
  assertThrows,
  runTests,
} from "./test_utils.ts";
import { tokenize } from "../src/lexer.ts";
import { parse } from "../src/parser.ts";
import { ParseError, ReservedWordError, LexerError } from "../src/errors.ts";

// =============================================================================
// Basic parsing
// =============================================================================

test("Parser - basic parsing", async (t) => {
  await t.step("empty template", () => {
    const tokens = tokenize("");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 0);
  });

  await t.step("plain text", () => {
    const tokens = tokenize("Hello, world!");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    assertEquals(ast.nodes[0].type, "text");
  });

  await t.step("simple variable", () => {
    const tokens = tokenize("{[ name ]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    assertEquals(ast.nodes[0].type, "variable");
    if (ast.nodes[0].type === "variable") {
      assertEquals(ast.nodes[0].path.segments, ["name"]);
    }
  });

  await t.step("dotted path variable", () => {
    const tokens = tokenize("{[ user.name ]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    if (ast.nodes[0].type === "variable") {
      assertEquals(ast.nodes[0].path.segments, ["user", "name"]);
    }
  });

  await t.step("deep path", () => {
    const tokens = tokenize("{[ a.b.c.d ]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "variable") {
      assertEquals(ast.nodes[0].path.segments, ["a", "b", "c", "d"]);
    }
  });
});

// =============================================================================
// If blocks
// =============================================================================

test("Parser - if blocks", async (t) => {
  await t.step("simple if block", () => {
    const tokens = tokenize("{[#if visible]}content{[/if]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    assertEquals(ast.nodes[0].type, "if_block");
    if (ast.nodes[0].type === "if_block") {
      assertEquals(ast.nodes[0].condition.segments, ["visible"]);
      assertEquals(ast.nodes[0].thenBranch.length, 1);
      assertEquals(ast.nodes[0].elseBranch, null);
    }
  });

  await t.step("if with else", () => {
    const tokens = tokenize("{[#if show]}yes{[#else]}no{[/if]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "if_block") {
      assertEquals(ast.nodes[0].thenBranch.length, 1);
      assertTrue(ast.nodes[0].elseBranch !== null);
      assertEquals(ast.nodes[0].elseBranch!.length, 1);
    }
  });

  await t.step("nested if blocks", () => {
    const tokens = tokenize("{[#if a]}{[#if b]}inner{[/if]}{[/if]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    if (ast.nodes[0].type === "if_block") {
      assertEquals(ast.nodes[0].thenBranch.length, 1);
      assertEquals(ast.nodes[0].thenBranch[0].type, "if_block");
    }
  });

  await t.step("if with path condition", () => {
    const tokens = tokenize("{[#if user.active]}active{[/if]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "if_block") {
      assertEquals(ast.nodes[0].condition.segments, ["user", "active"]);
    }
  });
});

// =============================================================================
// Unless blocks
// =============================================================================

test("Parser - unless blocks", async (t) => {
  await t.step("simple unless block", () => {
    const tokens = tokenize("{[#unless hidden]}visible{[/unless]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    assertEquals(ast.nodes[0].type, "unless_block");
    if (ast.nodes[0].type === "unless_block") {
      assertEquals(ast.nodes[0].condition.segments, ["hidden"]);
      assertEquals(ast.nodes[0].body.length, 1);
    }
  });

  await t.step("unless with variable in body", () => {
    const tokens = tokenize("{[#unless error]}{[ message ]}{[/unless]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "unless_block") {
      assertEquals(ast.nodes[0].body.length, 1);
      assertEquals(ast.nodes[0].body[0].type, "variable");
    }
  });
});

// =============================================================================
// Each blocks
// =============================================================================

test("Parser - each blocks", async (t) => {
  await t.step("simple each block", () => {
    const tokens = tokenize("{[#each items as item]}{[ item ]}{[/each]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    assertEquals(ast.nodes[0].type, "each_block");
    if (ast.nodes[0].type === "each_block") {
      assertEquals(ast.nodes[0].collection.segments, ["items"]);
      assertEquals(ast.nodes[0].itemName, "item");
    }
  });

  await t.step("nested each blocks", () => {
    const tokens = tokenize("{[#each rows as row]}{[#each row.cells as cell]}{[ cell ]}{[/each]}{[/each]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "each_block") {
      assertEquals(ast.nodes[0].body.length, 1);
      assertEquals(ast.nodes[0].body[0].type, "each_block");
    }
  });

  await t.step("each with path collection", () => {
    const tokens = tokenize("{[#each data.items as item]}{[ item ]}{[/each]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "each_block") {
      assertEquals(ast.nodes[0].collection.segments, ["data", "items"]);
    }
  });
});

// =============================================================================
// Unsecure output
// =============================================================================

test("Parser - unsecure output", async (t) => {
  await t.step("simple unsecure", () => {
    const tokens = tokenize("{[!unsecure html ]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    assertEquals(ast.nodes[0].type, "unsecure_output");
    if (ast.nodes[0].type === "unsecure_output") {
      assertEquals(ast.nodes[0].path.segments, ["html"]);
    }
  });

  await t.step("unsecure with path", () => {
    const tokens = tokenize("{[!unsecure user.bio ]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "unsecure_output") {
      assertEquals(ast.nodes[0].path.segments, ["user", "bio"]);
    }
  });
});

// =============================================================================
// Include
// =============================================================================

test("Parser - include", async (t) => {
  await t.step("simple include", () => {
    const tokens = tokenize("{[!include /header ]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    assertEquals(ast.nodes[0].type, "include");
    if (ast.nodes[0].type === "include") {
      assertEquals(ast.nodes[0].name, "/header");
      assertEquals(ast.nodes[0].args.length, 0);
    }
  });

  await t.step("include with arguments", () => {
    const tokens = tokenize("{[!include /card title=item.name ]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "include") {
      assertEquals(ast.nodes[0].name, "/card");
      assertEquals(ast.nodes[0].args.length, 1);
      assertEquals(ast.nodes[0].args[0].key, "title");
      assertEquals(ast.nodes[0].args[0].value.segments, ["item", "name"]);
    }
  });

  await t.step("include with nested path", () => {
    const tokens = tokenize("{[!include /shared/footer ]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "include") {
      assertEquals(ast.nodes[0].name, "/shared/footer");
    }
  });

  await t.step("include with multiple arguments", () => {
    const tokens = tokenize("{[!include /btn label=text href=url ]}");
    const ast = parse(tokens);
    if (ast.nodes[0].type === "include") {
      assertEquals(ast.nodes[0].args.length, 2);
      assertEquals(ast.nodes[0].args[0].key, "label");
      assertEquals(ast.nodes[0].args[1].key, "href");
    }
  });
});

// =============================================================================
// Error cases
// =============================================================================

test("Parser - error cases", async (t) => {
  await t.step("reserved word as variable throws ReservedWordError", () => {
    const tokens = tokenize("{[ if ]}");
    assertThrows(() => parse(tokens), ReservedWordError);
  });

  await t.step("reserved word 'each' throws", () => {
    const tokens = tokenize("{[ each ]}");
    assertThrows(() => parse(tokens), ReservedWordError);
  });

  await t.step("reserved word 'true' throws", () => {
    const tokens = tokenize("{[ true ]}");
    assertThrows(() => parse(tokens), ReservedWordError);
  });

  await t.step("reserved word 'false' throws", () => {
    const tokens = tokenize("{[ false ]}");
    assertThrows(() => parse(tokens), ReservedWordError);
  });

  await t.step("reserved word 'null' throws", () => {
    const tokens = tokenize("{[ null ]}");
    assertThrows(() => parse(tokens), ReservedWordError);
  });

  await t.step("identifier with underscore throws at lexer", () => {
    // Note: Underscore identifiers are caught by the lexer
    assertThrows(() => tokenize("{[ _private ]}"), LexerError);
  });

  await t.step("path segment with underscore throws at lexer", () => {
    // Note: Underscore identifiers are caught by the lexer
    assertThrows(() => tokenize("{[ user._id ]}"), LexerError);
  });

  await t.step("each item with underscore throws at lexer", () => {
    // Note: Underscore identifiers are caught by the lexer
    assertThrows(() => tokenize("{[#each items as _item]}{[/each]}"), LexerError);
  });

  await t.step("duplicate include argument throws", () => {
    const tokens = tokenize("{[!include /btn a=x a=y ]}");
    assertThrows(() => parse(tokens), ParseError, "Duplicate");
  });
});

// =============================================================================
// Mixed content
// =============================================================================

test("Parser - mixed content", async (t) => {
  await t.step("text and variable", () => {
    const tokens = tokenize("Hello, {[ name ]}!");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 3);
    assertEquals(ast.nodes[0].type, "text");
    assertEquals(ast.nodes[1].type, "variable");
    assertEquals(ast.nodes[2].type, "text");
  });

  await t.step("multiple blocks", () => {
    const tokens = tokenize("{[#if a]}A{[/if]}{[#if b]}B{[/if]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 2);
    assertEquals(ast.nodes[0].type, "if_block");
    assertEquals(ast.nodes[1].type, "if_block");
  });

  await t.step("if with each inside", () => {
    const tokens = tokenize("{[#if items]}{[#each items as item]}{[ item ]}{[/each]}{[/if]}");
    const ast = parse(tokens);
    assertEquals(ast.nodes.length, 1);
    if (ast.nodes[0].type === "if_block") {
      assertEquals(ast.nodes[0].thenBranch.length, 1);
      assertEquals(ast.nodes[0].thenBranch[0].type, "each_block");
    }
  });
});

// Run tests
runTests();
