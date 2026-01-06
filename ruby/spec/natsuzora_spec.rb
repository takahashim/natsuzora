# frozen_string_literal: true

RSpec.describe Natsuzora do
  describe '.render' do
    it 'renders simple template' do
      result = described_class.render('Hello, {[ name ]}!', { name: 'World' })
      expect(result).to eq('Hello, World!')
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

  describe 'integration' do
    it 'handles full pagination example from spec' do
      template = <<~TMPL
        <nav>
        {[#if pagination.has_prev]}<a href="{[ pagination.prev_url ]}">Prev</a>{[/if]}
        {[#each pagination.pages as page]}{[#if page.current]}<span>{[ page.num ]}</span>{[#else]}<a href="{[ page.url ]}">{[ page.num ]}</a>{[/if]}{[/each]}
        {[#if pagination.has_next]}<a href="{[ pagination.next_url ]}">Next</a>{[/if]}
        </nav>
      TMPL

      data = {
        pagination: {
          has_prev: true,
          has_next: true,
          prev_url: '/works?page=4',
          next_url: '/works?page=6',
          pages: [
            { num: 4, url: '/works?page=4', current: false },
            { num: 5, url: nil, current: true },
            { num: 6, url: '/works?page=6', current: false }
          ]
        }
      }

      result = described_class.render(template, data)

      expect(result).to include('<a href="/works?page=4">Prev</a>')
      expect(result).to include('<span>5</span>')
      expect(result).to include('<a href="/works?page=6">Next</a>')
    end
  end
end
