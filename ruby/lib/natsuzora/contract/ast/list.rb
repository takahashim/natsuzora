# frozen_string_literal: true

require_relative 'node'

module Natsuzora
  module Contract
    module AST
      # Ordered list of items of a single contract type.
      # Serialized as 'kind' => 'array' for JSON compatibility.
      class List < Node
        attr_reader :items

        def initialize(items)
          super()
          @items = items
        end

        def to_h
          { 'kind' => 'array', 'items' => @items.to_h }
        end

        def ==(other)
          other.is_a?(List) && other.items == @items
        end
      end
    end
  end
end
