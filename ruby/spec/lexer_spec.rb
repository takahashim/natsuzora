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
        expect(tokens.first).to have_attributes(type: :text, value: 'Hello World')
      end

      it 'returns only text and eof for plain text' do
        expect(token_types('Hello')).to eq(%i[text eof])
      end
    end

    context 'with variable expressions' do
      it 'tokenizes simple variable' do
        types = token_types('{[ name ]}')
        expect(types).to eq(%i[open whitespace ident whitespace close eof])
      end

      it 'tokenizes variable without spaces' do
        types = token_types('{[name]}')
        expect(types).to eq(%i[open ident close eof])
      end

      it 'tokenizes path with dots' do
        tokens = tokenize('{[ user.name ]}')
        idents = tokens.select { |t| t.type == :ident }.map(&:value)
        expect(idents).to eq(%w[user name])
      end

      it 'tokenizes nested path' do
        tokens = tokenize('{[ a.b.c.d ]}')
        idents = tokens.select { |t| t.type == :ident }.map(&:value)
        expect(idents).to eq(%w[a b c d])
      end
    end

    context 'with if blocks' do
      it 'tokenizes if block' do
        types = token_types('{[#if visible]}content{[/if]}')
        expect(types).to include(:hash, :kw_if, :slash)
      end

      it 'tokenizes if-else block' do
        types = token_types('{[#if x]}yes{[#else]}no{[/if]}')
        expect(types).to include(:kw_if, :kw_else)
      end
    end

    context 'with each blocks' do
      it 'tokenizes each block' do
        types = token_types('{[#each items as item]}{[/each]}')
        expect(types).to include(:kw_each, :kw_as)
      end

      it 'tokenizes each with index' do
        types = token_types('{[#each items as item, i]}{[/each]}')
        expect(types).to include(:comma)
      end
    end

    context 'with unsecure blocks' do
      it 'tokenizes unsecure block' do
        types = token_types('{[#unsecure]}raw{[/unsecure]}')
        expect(types).to include(:kw_unsecure)
      end
    end

    context 'with include' do
      it 'tokenizes include' do
        types = token_types('{[> /card]}')
        expect(types).to include(:gt, :ident)
      end

      it 'tokenizes include with nested path' do
        tokens = tokenize('{[> /components/card]}')
        name = tokens.find { |t| t.type == :ident && t.value.start_with?('/') }
        expect(name.value).to eq('/components/card')
      end

      it 'tokenizes include with arguments' do
        tokens = tokenize('{[> /card title=value]}')
        expect(tokens.map(&:type)).to include(:equal)
      end
    end

    context 'with mixed content' do
      it 'tokenizes text and variables' do
        types = token_types('Hello, {[ name ]}!')
        expect(types).to eq(%i[text open whitespace ident whitespace close text eof])
      end
    end

    context 'with comments' do
      it 'skips comment entirely' do
        types = token_types('{[! this is a comment ]}')
        expect(types).to eq(%i[eof])
      end

      it 'preserves text around comments' do
        types = token_types('before{[! comment ]}after')
        expect(types).to eq(%i[text text eof])
      end

      it 'handles comment without spaces' do
        types = token_types('{[!comment]}')
        expect(types).to eq(%i[eof])
      end

      it 'handles multi-line comment' do
        types = token_types("{[! multi\nline\ncomment ]}")
        expect(types).to eq(%i[eof])
      end

      it 'raises error for unclosed comment' do
        expect { tokenize('{[! unclosed') }.to raise_error(Natsuzora::LexerError, /Unclosed comment/)
      end
    end

    context 'with whitespace control' do
      it 'strips trailing whitespace with {[-' do
        tokens = tokenize("line1\n  {[- name ]}")
        text = tokens.find { |t| t.type == :text }
        expect(text.value).to eq("line1\n")
      end

      it 'strips newline with -]}' do
        tokens = tokenize("{[ name -]}\nnext")
        texts = tokens.select { |t| t.type == :text }
        expect(texts.map(&:value)).to eq(['next'])
      end

      it 'strips both sides with {[- ... -]}' do
        tokens = tokenize("before\n  {[- name -]}\nafter")
        texts = tokens.select { |t| t.type == :text }
        expect(texts.map(&:value)).to eq(["before\n", 'after'])
      end

      it 'does not strip if non-whitespace before {[-' do
        tokens = tokenize("text {[- name ]}")
        text = tokens.find { |t| t.type == :text }
        expect(text.value).to eq('text ')
      end

      it 'does not strip if non-whitespace after -]}' do
        tokens = tokenize("{[ name -]} more\nnext")
        texts = tokens.select { |t| t.type == :text }
        expect(texts.map(&:value)).to eq([" more\nnext"])
      end

      it 'handles {[- with block keywords' do
        types = token_types("{[-#if x -]}")
        expect(types).to include(:hash, :kw_if)
      end
    end

    context 'with delimiter escape' do
      it 'outputs {[ as text for {[{]}' do
        tokens = tokenize('{[{]}')
        expect(tokens.first).to have_attributes(type: :text, value: '{[')
      end

      it 'handles delimiter escape with surrounding text' do
        tokens = tokenize('Template syntax: {[{]} name ]}')
        texts = tokens.select { |t| t.type == :text }
        expect(texts.map(&:value)).to eq(['Template syntax: ', '{[', ' name ]}'])
      end

      it 'handles multiple delimiter escapes' do
        tokens = tokenize('{[{]} and {[{]}')
        texts = tokens.select { |t| t.type == :text }
        expect(texts.map(&:value)).to eq(['{[', ' and ', '{['])
      end

      it 'handles delimiter escape followed by variable' do
        tokens = tokenize('{[{]}{[ name ]}')
        expect(tokens.first).to have_attributes(type: :text, value: '{[')
        expect(tokens.map(&:type)).to include(:open, :ident, :close)
      end

      it 'raises error for incomplete delimiter escape' do
        expect { tokenize('{[{') }.to raise_error(Natsuzora::LexerError, /Expected '\]\}' after '\{\[\{'/)
      end

      it 'raises error for delimiter escape without close' do
        expect { tokenize('{[{ more text') }.to raise_error(Natsuzora::LexerError, /Expected '\]\}' after '\{\[\{'/)
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
        name_token = tokens.find { |t| t.type == :ident }
        expect(name_token.line).to eq(2)
      end

      it 'tracks column numbers' do
        tokens = tokenize('abc{[ name ]}')
        open_token = tokens.find { |t| t.type == :open }
        expect(open_token.column).to eq(4)
      end
    end
  end
end
