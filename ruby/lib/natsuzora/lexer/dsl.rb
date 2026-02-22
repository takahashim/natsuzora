# frozen_string_literal: true

require 'lexer_kit'

module Natsuzora
  class Lexer
    # LexerKit DSL definition for Natsuzora template language
    module DSL
      def self.build
        LexerKit.build do
          # Template syntax: text until {[, then tag mode
          # escape: "{[{]}" prevents delimiter detection (content retained as-is, needs post-processing)
          delimited :TEXT, delimiter: '{[', escape: '{[{]}' do
            # Comment marker
            token :PERCENT, '%'

            # Whitespace control marker
            token :DASH, '-'

            # Closing delimiter
            token :CLOSE, ']}', pop: true

            # Block markers
            token :HASH, '#'
            token :SLASH, '/'

            # Bang keywords (longest match takes priority over EXCLAMATION)
            token :BANG_UNSECURE, '!unsecure'
            token :BANG_INCLUDE, '!include'
            token :EXCLAMATION, '!'

            # Keywords
            token :KW_IF, 'if'
            token :KW_UNLESS, 'unless'
            token :KW_ELSE, 'else'
            token :KW_EACH, 'each'
            token :KW_AS, 'as'

            # Operators
            token :DOT, '.'
            token :COMMA, ','
            token :EQUAL, '='
            token :QUESTION, '?'

            # Whitespace
            token :WHITESPACE, /[ \t\r\n]+/

            # Identifiers
            token :IDENT, /[A-Za-z][A-Za-z0-9_]*/
          end
        end.compile
      end

      def self.instance
        @instance ||= build
      end
    end
  end
end
