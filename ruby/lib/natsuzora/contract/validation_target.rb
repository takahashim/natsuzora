# frozen_string_literal: true

module Natsuzora
  module Contract
    # Validation target for 2-generation contracts.
    module ValidationTarget
      # Validate against current generation (default)
      CURRENT = :current
      # Validate against next generation
      NEXT = :next
    end
  end
end
