# frozen_string_literal: true

RSpec.describe Natsuzora::Lexer::TokenProcessor do
  def tok(type, value, line: 1, column: 1)
    Natsuzora::Token.new(type, value, line: line, column: column)
  end

  def process(tokens)
    described_class.new(tokens).process
  end

  def types_and_values(result)
    result.map { |t| [t.type, t.value] }
  end

  describe 'comment with right trim' do
    it 'strips leading whitespace from next text after {[% comment -]}' do
      tokens = [
        tok(:TEXT, 'before'),
        tok(:PERCENT, '%'),
        tok(:WHITESPACE, ' '),
        tok(:IDENT, 'comment'),
        tok(:WHITESPACE, ' '),
        tok(:DASH, '-'),
        tok(:CLOSE, ']}'),
        tok(:TEXT, "\n  after")
      ]

      result = process(tokens)
      expect(types_and_values(result)).to eq([[:TEXT, 'before'], [:TEXT, '  after']])
    end
  end

  describe 'process_tag when CLOSE is missing' do
    it 'consumes all remaining tokens' do
      tokens = [
        tok(:TEXT, 'hello'),
        tok(:HASH, '#'),
        tok(:KW_IF, 'if'),
        tok(:WHITESPACE, ' '),
        tok(:IDENT, 'x')
      ]

      result = process(tokens)
      # TEXT is appended, then all remaining tokens (no CLOSE) are emitted
      expect(result.map(&:type)).to eq(%i[TEXT HASH KW_IF WHITESPACE IDENT])
    end
  end

  describe 'comment_tag? with empty tag_tokens' do
    it 'handles empty input without error' do
      # When find_close_index returns nil at start and no tokens remain,
      # tag_tokens would be an empty slice. Simulate by having only TEXT.
      tokens = [tok(:TEXT, 'just text')]
      result = process(tokens)
      expect(types_and_values(result)).to eq([[:TEXT, 'just text']])
    end
  end

  describe 'emit_tag_tokens filters DASH' do
    it 'removes DASH tokens from output for left-trimmed variable' do
      tokens = [
        tok(:TEXT, "line1\n  "),
        tok(:DASH, '-'),
        tok(:WHITESPACE, ' '),
        tok(:IDENT, 'name'),
        tok(:WHITESPACE, ' '),
        tok(:CLOSE, ']}')
      ]

      result = process(tokens)
      result_types = result.map(&:type)
      expect(result_types).not_to include(:DASH)
      expect(result_types).to eq(%i[TEXT WHITESPACE IDENT WHITESPACE CLOSE])
    end

    it 'removes DASH tokens from output for right-trimmed variable' do
      tokens = [
        tok(:WHITESPACE, ' '),
        tok(:IDENT, 'name'),
        tok(:WHITESPACE, ' '),
        tok(:DASH, '-'),
        tok(:CLOSE, ']}'),
        tok(:TEXT, "\nnext")
      ]

      result = process(tokens)
      result_types = result.map(&:type)
      expect(result_types).not_to include(:DASH)
    end
  end

  describe 'unclosed comment raises error' do
    it 'raises LexerError when comment has no CLOSE' do
      tokens = [
        tok(:PERCENT, '%', line: 1, column: 3),
        tok(:WHITESPACE, ' '),
        tok(:IDENT, 'comment')
      ]

      expect { process(tokens) }.to raise_error(Natsuzora::LexerError, /Unclosed comment/)
    end
  end
end
