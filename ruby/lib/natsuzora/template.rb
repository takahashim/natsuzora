# frozen_string_literal: true

module Natsuzora
  class Template
    attr_reader :ast

    def initialize(source, include_root: nil)
      @source = source
      @include_root = include_root

      Natsuzora.send(:require_ruby!)
      @ast = parse_ruby(source)
    end

    def render(data)
      loader = @include_root ? TemplateLoader.new(@include_root) : nil
      Renderer.new(@ast, template_loader: loader).render(data)
    end

    private

    def parse_ruby(source)
      tokens = Lexer.new(source).tokenize
      Parser.new(tokens).parse
    end
  end
end
