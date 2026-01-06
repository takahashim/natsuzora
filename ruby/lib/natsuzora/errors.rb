# frozen_string_literal: true

module Natsuzora
  class Error < StandardError
    attr_reader :line, :column

    def initialize(message, line: nil, column: nil)
      @line = line
      @column = column
      super(build_message(message))
    end

    private

    def build_message(message)
      return message unless line

      column ? "#{message} at line #{line}, column #{column}" : "#{message} at line #{line}"
    end
  end

  class LexerError < Error; end

  class ParseError < Error; end

  class ReservedWordError < ParseError; end

  class RenderError < Error; end

  class UndefinedVariableError < RenderError; end

  class TypeError < RenderError; end

  class IncludeError < RenderError; end

  class ShadowingError < RenderError; end
end
