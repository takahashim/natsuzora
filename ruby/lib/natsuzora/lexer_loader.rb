# frozen_string_literal: true

require 'lexer_kit'

module Natsuzora
  module LexerLoader
    def self.load_compiled(path)
      unless File.file?(path)
        raise LoadError, "Precompiled lexer is missing: #{path}. Run `rake lexers:compile`."
      end

      LexerKit.load_lexer(path)
    end
  end
end
