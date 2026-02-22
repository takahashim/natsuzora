# frozen_string_literal: true

module Natsuzora
  class Template
    attr_reader :ast

    def initialize(source, include_root: nil)
      @source = source
      @include_root = include_root

      return unless Natsuzora.backend == :ruby

      Natsuzora.send(:require_ruby!)
      @ast = parse_ruby(source)
    end

    def render(data)
      if Natsuzora.backend == :ffi
        Natsuzora.send(:require_ffi!)
        json_data = JSON.generate(data)
        FFI.render(@source, json_data, @include_root)
      else
        loader = @include_root ? TemplateLoader.new(@include_root) : nil
        Renderer.new(@ast, template_loader: loader).render(data)
      end
    end

    private

    def parse_ruby(source)
      tokens = Lexer.new(source).tokenize
      Parser.new(tokens).parse
    end
  end
end
