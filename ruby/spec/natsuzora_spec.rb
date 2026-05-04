# frozen_string_literal: true

RSpec.describe Natsuzora do
  describe '.render' do
    it 'raises TypeError when root data is an array' do
      expect { Natsuzora.render('Hello', []) }.to raise_error(Natsuzora::TypeError)
    end

    it 'raises TypeError when root data is a string' do
      expect { Natsuzora.render('Hello', 'not an object') }.to raise_error(Natsuzora::TypeError)
    end
  end

  describe '.parse' do
    it 'returns a template object' do
      template = Natsuzora.parse('{[ name ]}')
      expect(template).to be_a(Natsuzora::Template)
    end

    it 'allows reusing parsed template' do
      template = Natsuzora.parse('Hello, {[ name ]}!')
      expect(template.render(Natsuzora::Payload.new({ name: 'Alice' }))).to eq('Hello, Alice!')
      expect(template.render(Natsuzora::Payload.new({ name: 'Bob' }))).to eq('Hello, Bob!')
    end
  end

  describe 'VERSION' do
    it 'is defined' do
      expect(Natsuzora::VERSION).to match(/\d+\.\d+\.\d+/)
    end
  end
end
