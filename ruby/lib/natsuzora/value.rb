# frozen_string_literal: true

module Natsuzora
  module Value
    INTEGER_MIN = -9_007_199_254_740_991
    INTEGER_MAX = 9_007_199_254_740_991

    class << self
      def stringify(value)
        case value
        when String
          value
        when Integer
          validate_integer_range!(value)
          value.to_s
        when NilClass
          ''
        when TrueClass, FalseClass
          raise TypeError, 'Cannot stringify boolean value'
        when Array
          raise TypeError, 'Cannot stringify array'
        when Hash
          raise TypeError, 'Cannot stringify object'
        else
          raise TypeError, "Cannot stringify value of type #{value.class}"
        end
      end

      def truthy?(value)
        case value
        when false, nil
          false
        when Integer
          value != 0
        when String, Array, Hash
          !value.empty?
        else
          true
        end
      end

      def ensure_array!(value)
        raise TypeError, "Expected array, got #{value.class}" unless value.is_a?(Array)

        value
      end

      private

      def validate_integer_range!(value)
        return if value.between?(INTEGER_MIN, INTEGER_MAX)

        raise TypeError, "Integer out of range: #{value}"
      end
    end
  end
end
