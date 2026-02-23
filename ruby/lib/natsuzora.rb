# frozen_string_literal: true

require_relative 'natsuzora/version'
require_relative 'natsuzora/errors'

module Natsuzora
  class << self
    def render(source, data, include_root: nil)
      require_ruby!
      Template.new(source, include_root: include_root).render(data)
    end

    def parse(source, include_root: nil)
      require_ruby!
      Template.new(source, include_root: include_root)
    end

    private

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
