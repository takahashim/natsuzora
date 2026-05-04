# frozen_string_literal: true

require_relative 'node'
require_relative '../modifier'
require_relative '../scalar_type'

module Natsuzora
  module Contract
    module AST
      # Scalar value with type and modifier.
      class Scalar < Node
        attr_reader :scalar_type, :modifier

        def initialize(scalar_type, modifier = Modifier::NONE)
          super()
          @scalar_type = scalar_type
          @modifier = modifier
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
