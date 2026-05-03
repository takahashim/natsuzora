# frozen_string_literal: true

module Natsuzora
  # Wraps raw host data prepared for a single {Template#render} call.
  #
  # The class is the explicit boundary between untrusted host data
  # (Symbol-keyed Hashes, host-side numeric types, etc.) and the value
  # space that {Renderer} and {Context} consume.
  #
  # On construction:
  #
  # 1. Adapts host data via the {DataNormalizable} mixin
  #    (Symbol→String keys, whole-number Float→Integer). Pure
  #    transformation; never raises.
  # 2. Asserts conformance to Natsuzora's value type system via
  #    {Validator.validate_data!}. Raises {Natsuzora::TypeError} on any
  #    residual violation (Float left over, Integer outside the safe
  #    range, NaN/Infinity).
  #
  # If `new` returns, `#data` is guaranteed to conform; downstream
  # components trust the result without further validation.
  class Payload
    include DataNormalizable

    # @return [Hash] adapted and validated root data
    attr_reader :data

    def initialize(raw_data)
      raise Natsuzora::TypeError, 'Root data must be an object' unless raw_data.is_a?(Hash)

      @data = normalize_data(raw_data)
      Validator.validate_data!(@data)
    end
  end
end
