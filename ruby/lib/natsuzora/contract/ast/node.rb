# frozen_string_literal: true

module Natsuzora
  module Contract
    module AST
      # Base class for contract AST nodes.
      # Each subclass implements +to_h+ for JSON serialization.
      # Use +Natsuzora::Contract::AST.from_h+ for deserialization.
      class Node
        def to_h
          raise NotImplementedError
        end
      end
    end
  end
end
