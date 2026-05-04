# frozen_string_literal: true

module Natsuzora
  module Contract
    # Scalar type enumeration.
    module ScalarType
      # String type only
      STRING = :string
      # Integer type only
      INTEGER = :integer
      # Boolean type only (for truthiness checks)
      BOOL = :bool
      # String or Integer (stringifiable values, does NOT include bool)
      SCALAR = :scalar
    end
  end
end
