# frozen_string_literal: true

module Natsuzora
  class Token
    attr_reader :type, :value, :line, :column

    RESERVED_WORDS = %w[if unless else each as in of unsecure true false null include].freeze

    def initialize(type, value, line:, column:)
      @type = type
      @value = value
      @line = line
      @column = column
    end

    def inspect
      "#<Token #{type}:#{value.inspect} at #{line}:#{column}>"
    end
  end
end
