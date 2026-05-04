# frozen_string_literal: true

module Natsuzora
  module Contract
    # Diff marker for 2-generation contracts.
    module DiffMarker
      # `+` - Field will be added in next generation
      ADDED = :added
      # `-` - Field will be removed in next generation
      REMOVED = :removed
      # `*` - Field type will change in next generation
      CHANGED = :changed
    end
  end
end
