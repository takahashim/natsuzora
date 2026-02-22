# frozen_string_literal: true

require_relative 'lexer/dsl'
require_relative 'lexer/token_processor'
require_relative 'token'
require_relative 'errors'

module Natsuzora
  # Lexer for Natsuzora template language using LexerKit
  #
  # Responsibilities:
  # - Escape sequence processing ({[{]} -> {[)
  # - Whitespace control via TokenProcessor ({[- and -]})
  # - Error handling for invalid characters
  # - EOF token addition
  class Lexer
    ESCAPE_SEQUENCE = '{[{]}'
    ESCAPED_VALUE = '{['

    def initialize(source)
      @source = source
    end

    def tokenize
      stream = DSL.instance.stream(@source)
      mapped_tokens = map_tokens_from_stream(stream)
      processed_tokens = TokenProcessor.new(mapped_tokens).process
      add_eof(processed_tokens)
      processed_tokens
    end

    private

    def map_tokens_from_stream(stream)
      result = []

      until stream.eof?
        name = stream.token_name
        text = stream.text

        case name
        when :TEXT
          text_value = process_text_value(text)
          unless text_value.empty?
            line, col = stream.line_col
            result << Token.new(:TEXT, text_value, line: line, column: col)
          end

        when :INVALID
          line, col = stream.line_col
          raise LexerError.new("Unexpected character: '#{text}'", line: line, column: col)

        else
          line, col = stream.line_col
          result << Token.new(name, text, line: line, column: col)
        end

        stream.advance
      end

      result
    end

    def process_text_value(text)
      text.gsub(ESCAPE_SEQUENCE, ESCAPED_VALUE)
    end

    def add_eof(tokens)
      if tokens.empty?
        tokens << Token.new(:EOF, nil, line: 1, column: 1)
      else
        last = tokens.last
        line, column = position_after_value(last)
        tokens << Token.new(:EOF, nil, line: line, column: column)
      end
    end

    def position_after_value(token)
      line = token.line
      column = token.column
      value = token.value || ''

      value.each_char do |char|
        if char == "\n"
          line += 1
          column = 1
        else
          column += 1
        end
      end

      [line, column]
    end
  end
end
