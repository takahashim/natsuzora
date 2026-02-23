# frozen_string_literal: true

require_relative 'natsuzora/version'
require_relative 'natsuzora/errors'
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
require_relative 'natsuzora/template'

module Natsuzora
  class << self
    def render(source, data, include_root: nil)
      Template.new(source, include_root: include_root).render(data)
    end

    def parse(source, include_root: nil)
      Template.new(source, include_root: include_root)
    end
  end
end
