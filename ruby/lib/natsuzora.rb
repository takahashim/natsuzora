# frozen_string_literal: true

require_relative 'natsuzora/version'
require_relative 'natsuzora/errors'

module Natsuzora
  BACKENDS = %i[ruby ffi].freeze

  class << self
    def backend
      @backend ||= ENV['NATSUZORA_BACKEND']&.to_sym || :ruby
    end

    def backend=(value)
      raise ArgumentError, "Unknown backend: #{value}" unless BACKENDS.include?(value)

      @backend = value
    end

    def render(source, data, include_root: nil)
      case backend
      when :ffi
        require_ffi!
        json_data = JSON.generate(data)
        FFI.render(source, json_data, include_root)
      when :ruby
        require_ruby!
        Template.new(source, include_root: include_root).render(data)
      end
    end

    def parse(source, include_root: nil)
      require_ruby! if backend == :ruby
      Template.new(source, include_root: include_root)
    end

    private

    def require_ffi!
      return if defined?(@ffi_loaded)

      require 'json'
      require_relative 'natsuzora/ffi'
      @ffi_loaded = true
    end

    def require_ruby!
      return if defined?(@ruby_loaded)

      require_relative 'natsuzora/token'
      require_relative 'natsuzora/validator'
      require_relative 'natsuzora/html_escape'
      require_relative 'natsuzora/value'
      require_relative 'natsuzora/ast'
      require_relative 'natsuzora/lexer'
      require_relative 'natsuzora/parser'
      require_relative 'natsuzora/context'
      require_relative 'natsuzora/template_loader'
      require_relative 'natsuzora/renderer'
      @ruby_loaded = true
    end
  end
end

require_relative 'natsuzora/template'
