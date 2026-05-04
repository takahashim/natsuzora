# frozen_string_literal: true

module Natsuzora
  module Contract
    # Modifier for null/empty handling.
    module Modifier
      # Default: null not allowed, empty string allowed
      NONE = :none
      # `?` modifier: null allowed
      NULLABLE = :nullable
      # `!` modifier: null not allowed, empty string not allowed
      REQUIRED = :required
    end
  end
end
