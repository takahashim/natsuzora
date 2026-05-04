# frozen_string_literal: true

require_relative 'node'

module Natsuzora
  module Contract
    module AST
      # Any value (unconstrained).
      class Any < Node
        def to_h
          { 'kind' => 'any' }
        end

        def ==(other)
          other.is_a?(Any)
        end
      end
    end
  end
end
