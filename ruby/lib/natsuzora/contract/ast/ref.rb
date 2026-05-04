# frozen_string_literal: true

require_relative 'node'

module Natsuzora
  module Contract
    module AST
      # Type reference (used during parsing, resolved before use).
      class Ref < Node
        attr_reader :name

        def initialize(name)
          super()
          @name = name
        end

        def to_h
          { 'kind' => 'ref', 'name' => @name }
        end

        def ==(other)
          other.is_a?(Ref) && other.name == @name
        end
      end
    end
  end
end
