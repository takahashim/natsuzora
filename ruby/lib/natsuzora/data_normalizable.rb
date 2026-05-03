# frozen_string_literal: true

module Natsuzora
  # Mixin that gives the including class a `normalize_data(data)` method.
  module DataNormalizable
    # performs only pure transformations:
    # - Symbol Hash keys are converted to Strings (recursively).
    # - Whole-number Float values are converted to Integers.
    def normalize_data(data)
      case data
      when Hash
        data.transform_keys(&:to_s).transform_values { |v| normalize_data(v) }
      when Array
        data.map { |v| normalize_data(v) }
      when Float
        normalize_float(data)
      else
        data
      end
    end

    private

    # Convert finite whole-number Float to Integer; pass through anything else.
    def normalize_float(value)
      return value.to_i if value.finite? && value == value.to_i

      value
    end
  end
end
