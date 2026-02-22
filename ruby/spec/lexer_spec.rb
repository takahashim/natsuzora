# frozen_string_literal: true

RSpec.describe Natsuzora::Lexer do
  def tokenize(source)
    described_class.new(source).tokenize
  end

  def token_types(source)
    tokenize(source).map(&:type)
  end

  describe '#tokenize' do
    context 'with plain text' do
      it 'tokenizes plain text' do
        tokens = tokenize('Hello World')
        expect(tokens.first).to have_attributes(type: :TEXT, value: 'Hello World')
      end

      it 'returns only text and eof for plain text' do
        expect(token_types('Hello')).to eq(%i[TEXT EOF])
      end
    end

    context 'with variable expressions' do
      # NOTE: The lexer does not emit :open token anymore.
      # The opening {[ is consumed by the delimited mode.
      it 'tokenizes simple variable' do
        types = token_types('{[ name ]}')
        expect(types).to eq(%i[WHITESPACE IDENT WHITESPACE CLOSE EOF])
      end

      it 'tokenizes variable without spaces' do
        types = token_types('{[name]}')
        expect(types).to eq(%i[IDENT CLOSE EOF])
      end

      it 'tokenizes path with dots' do
        tokens = tokenize('{[ user.name ]}')
        idents = tokens.select { |t| t.type == :IDENT }.map(&:value)
        expect(idents).to eq(%w[user name])
      end

      it 'tokenizes nested path' do
        tokens = tokenize('{[ a.b.c.d ]}')
        idents = tokens.select { |t| t.type == :IDENT }.map(&:value)
        expect(idents).to eq(%w[a b c d])
      end
    end

    context 'with if blocks' do
      it 'tokenizes if block' do
        types = token_types('{[#if visible]}content{[/if]}')
        expect(types).to include(:HASH, :KW_IF, :SLASH)
      end

      it 'tokenizes if-else block' do
        types = token_types('{[#if x]}yes{[#else]}no{[/if]}')
        expect(types).to include(:KW_IF, :KW_ELSE)
      end
    end

    context 'with each blocks' do
      it 'tokenizes each block' do
        types = token_types('{[#each items as item]}{[/each]}')
        expect(types).to include(:KW_EACH, :KW_AS)
      end

      it 'tokenizes each with index' do
        types = token_types('{[#each items as item, i]}{[/each]}')
        expect(types).to include(:COMMA)
      end
    end

    context 'with unsecure output' do
      it 'tokenizes unsecure output' do
        types = token_types('{[!unsecure html ]}')
        expect(types).to include(:BANG_UNSECURE)
      end
    end

    context 'with include' do
      it 'tokenizes include' do
        types = token_types('{[!include /card ]}')
        expect(types).to include(:BANG_INCLUDE, :SLASH, :IDENT)
      end

      it 'tokenizes include with nested path' do
        tokens = tokenize('{[!include /components/card ]}')
        # Include path is built from SLASH and IDENT tokens by the parser
        slashes = tokens.select { |t| t.type == :SLASH }
        idents = tokens.select { |t| t.type == :IDENT }.map(&:value)
        expect(slashes.size).to eq(2)
        expect(idents).to eq(%w[components card])
      end

      it 'tokenizes include with arguments' do
        tokens = tokenize('{[!include /card title=value ]}')
        expect(tokens.map(&:type)).to include(:EQUAL)
      end
    end

    context 'with mixed content' do
      # NOTE: No :open token anymore
      it 'tokenizes text and variables' do
        types = token_types('Hello, {[ name ]}!')
        expect(types).to eq(%i[TEXT WHITESPACE IDENT WHITESPACE CLOSE TEXT EOF])
      end
    end

    context 'with comments' do
      # NOTE: Comments are removed by TokenProcessor.
      # No :percent or comment content tokens appear in output.
      it 'removes comment tokens' do
        types = token_types('{[% this is a comment ]}')
        expect(types).not_to include(:PERCENT)
        expect(types).to eq(%i[EOF])
      end

      it 'preserves text around comments' do
        tokens = tokenize('before{[% comment ]}after')
        texts = tokens.select { |t| t.type == :TEXT }
        expect(texts.map(&:value)).to eq(%w[before after])
        expect(tokens.map(&:type)).not_to include(:PERCENT)
      end

      it 'handles comment without spaces' do
        types = token_types('{[%comment]}')
        expect(types).not_to include(:PERCENT)
        expect(types).to eq(%i[EOF])
      end

      it 'handles multi-line comment' do
        types = token_types("{[% multi\nline\ncomment ]}")
        expect(types).not_to include(:PERCENT)
        expect(types).to eq(%i[EOF])
      end

      it 'raises error for unclosed comment' do
        expect { tokenize('{[% unclosed') }.to raise_error(Natsuzora::LexerError, /Unclosed comment/)
      end
    end

    context 'with whitespace control markers' do
      # NOTE: TokenProcessor consumes DASH tokens and strips adjacent TEXT.
      # DASH tokens do not appear in output - they only affect text content.

      it 'strips trailing whitespace for {[-' do
        tokens = tokenize("line1\n  {[- name ]}")
        types = tokens.map(&:type)
        expect(types).not_to include(:DASH)
        # Text is stripped by TokenProcessor
        text = tokens.find { |t| t.type == :TEXT }
        expect(text.value).to eq("line1\n")
      end

      it 'strips leading whitespace and newline for -]}' do
        tokens = tokenize("{[ name -]}\nnext")
        types = tokens.map(&:type)
        expect(types).not_to include(:DASH)
        # Text is stripped by TokenProcessor
        texts = tokens.select { |t| t.type == :TEXT }
        expect(texts.map(&:value)).to eq(['next'])
      end

      it 'strips both sides for {[- ... -]}' do
        tokens = tokenize("before\n  {[- name -]}\nafter")
        dashes = tokens.select { |t| t.type == :DASH }
        expect(dashes.size).to eq(0)
        # Text is stripped by TokenProcessor
        texts = tokens.select { |t| t.type == :TEXT }
        expect(texts.map(&:value)).to eq(%W[before\n after])
      end

      it 'handles {[- with block keywords' do
        types = token_types('{[-#if x -]}')
        expect(types).not_to include(:DASH)
        expect(types).to include(:HASH, :KW_IF)
      end
    end

    context 'with delimiter escape' do
      it 'outputs {[ as text for {[{]}' do
        tokens = tokenize('{[{]}')
        expect(tokens.first).to have_attributes(type: :TEXT, value: '{[')
      end

      it 'handles delimiter escape with surrounding text' do
        tokens = tokenize('Template syntax: {[{]} name ]}')
        texts = tokens.select { |t| t.type == :TEXT }
        # LexerKit combines TEXT tokens with escape sequences
        expect(texts.map(&:value).join).to eq('Template syntax: {[ name ]}')
      end

      it 'handles multiple delimiter escapes' do
        tokens = tokenize('{[{]} and {[{]}')
        texts = tokens.select { |t| t.type == :TEXT }
        # LexerKit combines TEXT tokens with escape sequences
        expect(texts.map(&:value).join).to eq('{[ and {[')
      end

      it 'handles delimiter escape followed by variable' do
        tokens = tokenize('{[{]}{[ name ]}')
        expect(tokens.first).to have_attributes(type: :TEXT, value: '{[')
        # No :open token anymore
        expect(tokens.map(&:type)).to include(:IDENT, :CLOSE)
      end

      it 'raises error for incomplete delimiter escape' do
        # LexerKit raises error for invalid character { inside tag
        expect { tokenize('{[{') }.to raise_error(Natsuzora::LexerError, /Unexpected character/)
      end

      it 'raises error for delimiter escape without close' do
        # LexerKit raises error for invalid character { inside tag
        expect { tokenize('{[{ more text') }.to raise_error(Natsuzora::LexerError, /Unexpected character/)
      end
    end

    context 'with errors' do
      it 'raises error on unexpected character inside tag' do
        expect { tokenize('{[ @ ]}') }.to raise_error(Natsuzora::LexerError, /Unexpected character/)
      end
    end

    context 'with line and column tracking' do
      it 'tracks line numbers' do
        tokens = tokenize("line1\n{[ name ]}")
        name_token = tokens.find { |t| t.type == :IDENT }
        expect(name_token.line).to eq(2)
      end

      it 'tracks column numbers' do
        tokens = tokenize('abc{[ name ]}')
        # The first token after text is whitespace (the space after {[)
        # In the new architecture, {[ is consumed without emitting :open token
        # so the first emitted token inside the tag is at column 6 (1-indexed)
        non_text = tokens.find { |t| t.type != :TEXT }
        expect(non_text.column).to eq(6)
      end
    end
  end
end
