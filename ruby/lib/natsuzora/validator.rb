# frozen_string_literal: true

module Natsuzora
  # Centralized validation functions for Natsuzora templates
  module Validator
    class << self
      # Validate an identifier (variable name, each binding, include argument key)
      #
      # Rules:
      # - Cannot be a reserved word (if, unless, each, as, unsecure, true, false, null, include)
      # - Cannot start with '_' (reserved for internal use)
      # - Cannot contain '@' (reserved for future use)
      def validate_identifier!(name, line: nil, column: nil)
        raise ReservedWordError.new("'#{name}' is a reserved word", line: line, column: column) if Token::RESERVED_WORDS.include?(name)

        raise ParseError.new("Identifier cannot start with '_': #{name}", line: line, column: column) if name.start_with?('_')

        return unless name.include?('@')

        raise ParseError.new("Identifier cannot contain '@': #{name}", line: line, column: column)
      end

      # Validate an include name at parse time
      #
      # Lexer ensures each segment follows Identifier rules (starts with letter).
      # This validates additional constraints:
      # - Must start with '/'
      # - Must have at least one segment after '/'
      def validate_include_name_syntax!(name, line: nil, column: nil)
        raise ParseError.new("Include name must start with '/'", line: line, column: column) unless name.start_with?('/')

        return unless name == '/'

        raise ParseError.new('Include name must have at least one segment', line: line, column: column)
      end

      # Validate an include name at load time
      #
      # Defense in depth: re-check basic rules even though lexer enforces them
      def validate_include_name_runtime!(name)
        raise IncludeError, "Include name must start with '/': #{name}" unless name.start_with?('/')

        # These should be impossible with the new lexer, but check anyway
        return unless name.include?('..') || name.include?('//') || name.include?('\\') || name.include?(':')

        raise IncludeError, "Invalid include name: #{name}"
      end

      # Validate that runtime data conforms to Natsuzora's value type system.
      #
      # Error message:
      # - Float NaN / Infinity     → "Invalid number: ..."
      # - Other (non-whole) Float  → "Floating point numbers are not supported: ..."
      # - Integer out of safe range → "Integer out of range: ..."
      def validate_data!(data)
        case data
        when Hash
          data.each_value { |v| validate_data!(v) }
        when Array
          data.each { |v| validate_data!(v) }
        when Integer
          return if data.between?(Value::INTEGER_MIN, Value::INTEGER_MAX)

          raise Natsuzora::TypeError, "Integer out of range: #{data}"
        when Float
          raise Natsuzora::TypeError, "Invalid number: #{data}" unless data.finite?

          raise Natsuzora::TypeError, "Floating point numbers are not supported: #{data}"
        end
      end
    end
  end
end
