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
        if Token::RESERVED_WORDS.include?(name)
          raise ReservedWordError.new("'#{name}' is a reserved word", line: line, column: column)
        end

        if name.start_with?('_')
          raise ParseError.new("Identifier cannot start with '_': #{name}", line: line, column: column)
        end

        return unless name.include?('@')

        raise ParseError.new("Identifier cannot contain '@': #{name}", line: line, column: column)
      end

      # Validate an include name at parse time
      #
      # Rules:
      # - Must start with '/'
      # - Must have at least one segment after '/'
      # - Cannot contain '..' (path traversal)
      # - Cannot contain '//' (double slash)
      # - Cannot contain ':' (Windows drive letters)
      def validate_include_name_syntax!(name, line: nil, column: nil)
        unless name.start_with?('/')
          raise ParseError.new("Include name must start with '/'", line: line, column: column)
        end

        if name == '/'
          raise ParseError.new('Include name must have at least one segment', line: line, column: column)
        end

        raise ParseError.new("Include name cannot contain '..'", line: line, column: column) if name.include?('..')

        raise ParseError.new("Include name cannot contain '//'", line: line, column: column) if name.include?('//')

        return unless name.include?(':')

        raise ParseError.new("Include name cannot contain ':'", line: line, column: column)
      end

      # Validate an include name at load time
      #
      # Additional rules beyond parse-time validation:
      # - Cannot contain '\' (Windows path separator)
      def validate_include_name_runtime!(name)
        raise IncludeError, "Include name must start with '/': #{name}" unless name.start_with?('/')

        raise IncludeError, "Invalid include name: #{name}" if name.include?('..') || name.include?('//')

        return unless name.include?('\\') || name.include?(':')

        raise IncludeError, "Invalid include name: #{name}"
      end
    end
  end
end
