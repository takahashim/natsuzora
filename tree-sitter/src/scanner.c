/**
 * External scanner for Natsuzora tree-sitter grammar.
 *
 * Handles comment disambiguation: {[! ... ]} is a comment UNLESS
 * the content after ! starts with 'include' or 'unsecure' keyword
 * (which are handled by parser-level rules instead).
 */

#include "tree_sitter/parser.h"

#include <string.h>
#include <stdbool.h>

enum TokenType {
    COMMENT,
};

void *tree_sitter_natsuzora_external_scanner_create(void) {
    return NULL;
}

void tree_sitter_natsuzora_external_scanner_destroy(void *payload) {
    (void)payload;
}

unsigned tree_sitter_natsuzora_external_scanner_serialize(void *payload, char *buffer) {
    (void)payload;
    (void)buffer;
    return 0;
}

void tree_sitter_natsuzora_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
    (void)payload;
    (void)buffer;
    (void)length;
}

/**
 * Skip whitespace characters, returning the count of characters skipped.
 */
static int skip_ws(TSLexer *lexer) {
    int count = 0;
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
           lexer->lookahead == '\r' || lexer->lookahead == '\n') {
        lexer->advance(lexer, false);
        count++;
    }
    return count;
}

/**
 * Check if the upcoming characters match a given keyword followed by whitespace.
 * Does NOT advance the lexer (uses mark_end to preserve position).
 */
static bool check_keyword(TSLexer *lexer, const char *keyword) {
    for (int i = 0; keyword[i] != '\0'; i++) {
        if (lexer->lookahead != (uint32_t)keyword[i]) {
            return false;
        }
        lexer->advance(lexer, false);
    }
    /* Keyword must be followed by whitespace or end-of-tag marker */
    return lexer->lookahead == ' ' || lexer->lookahead == '\t' ||
           lexer->lookahead == '\r' || lexer->lookahead == '\n' ||
           lexer->lookahead == '-' || lexer->lookahead == ']';
}

bool tree_sitter_natsuzora_external_scanner_scan(
    void *payload,
    TSLexer *lexer,
    const bool *valid_symbols
) {
    (void)payload;

    if (!valid_symbols[COMMENT]) {
        return false;
    }

    /* Try to match {[ or {[- */
    if (lexer->lookahead != '{') return false;
    lexer->advance(lexer, false);

    if (lexer->lookahead != '[') return false;
    lexer->advance(lexer, false);

    /* Optional whitespace control marker '-' */
    if (lexer->lookahead == '-') {
        lexer->advance(lexer, false);
    }

    /* Must have '!' for comment/include/unsecure */
    if (lexer->lookahead != '!') return false;
    lexer->advance(lexer, false);

    /* Skip optional whitespace after '!' */
    skip_ws(lexer);

    /*
     * Check if this is actually an include or unsecure tag.
     * If so, return false so the parser uses the internal rules.
     *
     * We need to save position and check keywords. Since tree-sitter
     * external scanners can't "undo" advances, we use mark_end to
     * track what we've consumed and check the keyword characters.
     *
     * However, tree-sitter external scanners are greedy - once we
     * advance, we can't go back. So we need a different approach:
     * we check the first character to quickly rule out include/unsecure,
     * and if it matches, we check the full keyword by advancing through it.
     *
     * If it IS a keyword, we return false. But we've already advanced
     * past the keyword characters. This is OK because returning false
     * means the lexer restarts from the original position.
     */

    if (lexer->lookahead == 'i') {
        /* Could be 'include' */
        if (check_keyword(lexer, "include")) {
            return false;
        }
    } else if (lexer->lookahead == 'u') {
        /* Could be 'unsecure' */
        if (check_keyword(lexer, "unsecure")) {
            return false;
        }
    }

    /*
     * This is a comment. Consume everything until ]} or -]}.
     * We need to handle:
     *   - Regular close: ]}
     *   - Whitespace control close: -]}
     */
    while (lexer->lookahead != 0) {
        if (lexer->lookahead == '-') {
            lexer->advance(lexer, false);
            if (lexer->lookahead == ']') {
                lexer->advance(lexer, false);
                if (lexer->lookahead == '}') {
                    lexer->advance(lexer, false);
                    lexer->result_symbol = COMMENT;
                    return true;
                }
            }
        } else if (lexer->lookahead == ']') {
            lexer->advance(lexer, false);
            if (lexer->lookahead == '}') {
                lexer->advance(lexer, false);
                lexer->result_symbol = COMMENT;
                return true;
            }
        } else {
            lexer->advance(lexer, false);
        }
    }

    /* Unclosed comment - return false */
    return false;
}
