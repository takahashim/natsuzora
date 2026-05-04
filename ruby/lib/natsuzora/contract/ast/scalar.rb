# frozen_string_literal: true

require_relative 'node'
require_relative '../scalar_type'

module Natsuzora
  module Contract
    module AST
      # Scalar value with type and modifier.
      class Scalar < Node
        # Modifier for null/empty handling on scalar values.
        module Modifier
          # Default: null not allowed, empty string allowed
          NONE = :none
          # `?` modifier: null allowed
          NULLABLE = :nullable
          # `!` modifier: null not allowed, empty string not allowed
          REQUIRED = :required
        end

        attr_reader :scalar_type, :modifier

        def initialize(scalar_type, modifier = Modifier::NONE)
          super()
          @scalar_type = scalar_type
          @modifier = modifier
        end

        def nullable?
          @modifier == Modifier::NULLABLE
        end

        def required?
          @modifier == Modifier::REQUIRED
        end

        def accepts?(data)
          case @scalar_type
          when ScalarType::STRING then data.is_a?(String)
          when ScalarType::INTEGER then data.is_a?(Integer)
          when ScalarType::BOOL then data.is_a?(TrueClass) || data.is_a?(FalseClass)
          when ScalarType::SCALAR then data.is_a?(String) || data.is_a?(Integer)
          end
        end

        def to_h
          h = { 'kind' => 'scalar', 'type' => @scalar_type.to_s }
          h['modifier'] = @modifier.to_s unless @modifier == Modifier::NONE
          h
        end

        def ==(other)
          other.is_a?(Scalar) &&
            other.scalar_type == @scalar_type &&
            other.modifier == @modifier
        end
      end
    end
  end
end
