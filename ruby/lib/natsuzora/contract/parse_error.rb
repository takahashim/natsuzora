# frozen_string_literal: true

module Natsuzora
  module Contract
    # Error that can occur during parsing.
    class ParseError < StandardError
      attr_reader :line, :column

      def initialize(message, line = 0, column = 0)
        @line = line
        @column = column
        super("#{message} at line #{line}, column #{column}")
      end
    end
  end
end
