# frozen_string_literal: true

require_relative '../lexer_loader'

module Natsuzora
  class Lexer
    module CompiledLexer
      LEXER_PATH = File.expand_path('../data/lexers/template.lkt1', __dir__)

      def self.instance
        @instance ||= LexerLoader.load_compiled(LEXER_PATH)
      end
    end
  end
end
