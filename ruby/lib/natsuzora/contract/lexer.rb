# frozen_string_literal: true

require 'lexer_kit'
require_relative 'types'

module Natsuzora
  module Contract
    # LexerKit definition for Subaru tokens
    # Available tokens: LEXER.tokens
    LEXER = LexerKit.build do
      # Fixed string tokens (longer patterns first)
      token :ARROW, '->'
      token :COLON, ':'
      token :OPEN_BRACE, '{'
      token :CLOSE_BRACE, '}'
      token :OPEN_BRACKET, '['
      token :CLOSE_BRACKET, ']'
      token :QUESTION, '?'
      token :EXCLAMATION, '!'
      token :PLUS, '+'  # diff marker
      token :MINUS, '-' # diff marker
      token :STAR, '*'  # diff marker

      # Regex tokens
      token :COMMENT, /#[^\n]*/
      token :TYPE_NAME, /[A-Z][a-zA-Z0-9_]*/  # uppercase identifier (type name)
      token :IDENTIFIER, /[a-z][a-zA-Z0-9_]*/ # lowercase identifier (field name)
      token :NEWLINE, /\n/

      # Skip whitespace
      token :WS, /[ \t\r]+/, skip: true
    end.compile
  end
end
