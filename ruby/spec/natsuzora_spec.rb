# frozen_string_literal: true

RSpec.describe Natsuzora do
  describe '.render' do
    it 'raises TypeError when root data is an array' do
      expect { described_class.render('Hello', []) }.to raise_error(Natsuzora::TypeError)
    end

    it 'raises TypeError when root data is a string' do
      expect { described_class.render('Hello', 'not an object') }.to raise_error(Natsuzora::TypeError)
    end
  end

  describe '.parse' do
    it 'returns a template object' do
      template = described_class.parse('{[ name ]}')
      expect(template).to be_a(Natsuzora::Template)
    end

    it 'allows reusing parsed template' do
      template = described_class.parse('Hello, {[ name ]}!')
      expect(template.render({ name: 'Alice' })).to eq('Hello, Alice!')
      expect(template.render({ name: 'Bob' })).to eq('Hello, Bob!')
    end
  end

  describe 'VERSION' do
    it 'is defined' do
      expect(Natsuzora::VERSION).to match(/\d+\.\d+\.\d+/)
    end
  end

end
