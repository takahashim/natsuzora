# frozen_string_literal: true

RSpec.describe Natsuzora::Renderer do
  def render(source, data, include_root: nil)
    Natsuzora.render(source, data, include_root: include_root)
  end

  describe '#render' do
    context 'with plain text' do
      it 'returns text as-is' do
        expect(render('Hello World', {})).to eq('Hello World')
      end
    end

    context 'with variables' do
      it 'renders string variable' do
        expect(render('{[ name ]}', { name: 'Alice' })).to eq('Alice')
      end

      it 'renders integer variable' do
        expect(render('{[ count ]}', { count: 42 })).to eq('42')
      end

      it 'renders negative integer' do
        expect(render('{[ num ]}', { num: -123 })).to eq('-123')
      end

      it 'raises error for null without modifier (v4.0)' do
        expect { render('{[ value ]}', { value: nil }) }
          .to raise_error(Natsuzora::TypeError, /null/)
      end

      it 'renders null as empty string with ? modifier' do
        expect(render('{[ value? ]}', { value: nil })).to eq('')
      end

      it 'renders value with ? modifier' do
        expect(render('{[ value? ]}', { value: 'test' })).to eq('test')
      end

      it 'renders value with ! modifier' do
        expect(render('{[ value! ]}', { value: 'test' })).to eq('test')
      end

      it 'raises error for null with ! modifier' do
        expect { render('{[ value! ]}', { value: nil }) }
          .to raise_error(Natsuzora::TypeError, /null/)
      end

      it 'raises error for empty string with ! modifier' do
        expect { render('{[ value! ]}', { value: '' }) }
          .to raise_error(Natsuzora::TypeError, /empty/)
      end

      it 'renders nested path' do
        expect(render('{[ user.name ]}', { user: { name: 'Bob' } })).to eq('Bob')
      end

      it 'escapes HTML in output' do
        expect(render('{[ html ]}', { html: '<script>' })).to eq('&lt;script&gt;')
      end

      it 'escapes all special characters' do
        expect(render('{[ s ]}', { s: "&<>\"'" })).to eq('&amp;&lt;&gt;&quot;&#39;')
      end
    end

    context 'with undefined variables' do
      it 'raises error for undefined variable' do
        expect { render('{[ unknown ]}', {}) }
          .to raise_error(Natsuzora::UndefinedVariableError, /unknown/)
      end

      it 'raises error for undefined nested path' do
        expect { render('{[ user.missing ]}', { user: {} }) }
          .to raise_error(Natsuzora::UndefinedVariableError, /missing/)
      end
    end

    context 'with type errors' do
      it 'raises error for boolean stringification' do
        expect { render('{[ flag ]}', { flag: true }) }
          .to raise_error(Natsuzora::TypeError, /boolean/)
      end

      it 'raises error for array stringification' do
        expect { render('{[ arr ]}', { arr: [1, 2] }) }
          .to raise_error(Natsuzora::TypeError, /array/)
      end

      it 'raises error for object stringification' do
        expect { render('{[ obj ]}', { obj: { a: 1 } }) }
          .to raise_error(Natsuzora::TypeError, /object/)
      end
    end

    context 'with truthiness' do
      it 'treats false as falsy' do
        expect(render('{[#if x]}yes{[/if]}', { x: false })).to eq('')
      end

      it 'treats null as falsy' do
        expect(render('{[#if x]}yes{[/if]}', { x: nil })).to eq('')
      end

      it 'treats 0 as falsy' do
        expect(render('{[#if x]}yes{[/if]}', { x: 0 })).to eq('')
      end

      it 'treats empty string as falsy' do
        expect(render('{[#if x]}yes{[/if]}', { x: '' })).to eq('')
      end

      it 'treats empty array as falsy' do
        expect(render('{[#if x]}yes{[/if]}', { x: [] })).to eq('')
      end

      it 'treats empty object as falsy' do
        expect(render('{[#if x]}yes{[/if]}', { x: {} })).to eq('')
      end

      it 'treats non-zero as truthy' do
        expect(render('{[#if x]}yes{[/if]}', { x: 1 })).to eq('yes')
      end

      it 'treats non-empty string as truthy' do
        expect(render('{[#if x]}yes{[/if]}', { x: 'a' })).to eq('yes')
      end

      it 'treats non-empty array as truthy' do
        expect(render('{[#if x]}yes{[/if]}', { x: [1] })).to eq('yes')
      end

      it 'treats non-empty object as truthy' do
        expect(render('{[#if x]}yes{[/if]}', { x: { a: 1 } })).to eq('yes')
      end

      it 'treats true as truthy' do
        expect(render('{[#if x]}yes{[/if]}', { x: true })).to eq('yes')
      end
    end

    context 'with if-else' do
      it 'renders then branch when truthy' do
        expect(render('{[#if x]}yes{[#else]}no{[/if]}', { x: true })).to eq('yes')
      end

      it 'renders else branch when falsy' do
        expect(render('{[#if x]}yes{[#else]}no{[/if]}', { x: false })).to eq('no')
      end
    end

    context 'with unless blocks' do
      it 'renders content when condition is falsy' do
        expect(render('{[#unless hidden]}visible{[/unless]}', { hidden: false })).to eq('visible')
      end

      it 'renders nothing when condition is truthy' do
        expect(render('{[#unless hidden]}visible{[/unless]}', { hidden: true })).to eq('')
      end

      it 'treats null as falsy' do
        expect(render('{[#unless x]}shown{[/unless]}', { x: nil })).to eq('shown')
      end

      it 'treats 0 as falsy' do
        expect(render('{[#unless x]}shown{[/unless]}', { x: 0 })).to eq('shown')
      end

      it 'treats empty string as falsy' do
        expect(render('{[#unless x]}shown{[/unless]}', { x: '' })).to eq('shown')
      end

      it 'treats empty array as falsy' do
        expect(render('{[#unless x]}shown{[/unless]}', { x: [] })).to eq('shown')
      end

      it 'treats non-zero as truthy' do
        expect(render('{[#unless x]}shown{[/unless]}', { x: 1 })).to eq('')
      end

      it 'treats non-empty string as truthy' do
        expect(render('{[#unless x]}shown{[/unless]}', { x: 'a' })).to eq('')
      end
    end

    context 'with each blocks' do
      it 'iterates over array' do
        result = render('{[#each items as item]}{[ item ]}{[/each]}', { items: %w[a b c] })
        expect(result).to eq('abc')
      end

      it 'renders empty for empty array' do
        result = render('{[#each items as item]}x{[/each]}', { items: [] })
        expect(result).to eq('')
      end

      it 'allows nested path in item' do
        data = { items: [{ name: 'a' }, { name: 'b' }] }
        result = render('{[#each items as item]}{[ item.name ]}{[/each]}', data)
        expect(result).to eq('ab')
      end

      it 'raises error for non-array' do
        expect { render('{[#each x as item]}{[/each]}', { x: 'not array' }) }
          .to raise_error(Natsuzora::TypeError, /array/)
      end
    end

    context 'with unsecure output' do
      it 'does not escape unsecure output' do
        expect(render('{[!unsecure html ]}', { html: '<b>' })).to eq('<b>')
      end

      it 'escapes regular variables but not unsecure' do
        result = render('{[ a ]}{[!unsecure b ]}{[ c ]}', { a: '<', b: '<', c: '<' })
        expect(result).to eq('&lt;<&lt;')
      end
    end

    context 'with shadowing' do
      it 'raises error when shadowing root variable' do
        expect { render('{[#each items as name]}{[/each]}', { items: [1], name: 'x' }) }
          .to raise_error(Natsuzora::ShadowingError, /name/)
      end

      it 'raises error when shadowing outer each variable' do
        template = '{[#each a as x]}{[#each b as x]}{[/each]}{[/each]}'
        expect { render(template, { a: [1], b: [1] }) }
          .to raise_error(Natsuzora::ShadowingError, /x/)
      end
    end

    context 'with comments' do
      it 'ignores comment in output' do
        expect(render('Hello{[% comment ]}World', {})).to eq('HelloWorld')
      end

      it 'ignores comment with spaces' do
        expect(render('Hello {[% comment ]} World', {})).to eq('Hello  World')
      end

      it 'handles multi-line comments' do
        template = "Hello{[% this is\na multi-line\ncomment ]}World"
        expect(render(template, {})).to eq('HelloWorld')
      end

      it 'handles comment between variables' do
        expect(render('{[ a ]}{[% ignored ]}{[ b ]}', { a: '1', b: '2' })).to eq('12')
      end

      it 'handles comment inside blocks' do
        template = '{[#if x]}{[% comment ]}yes{[/if]}'
        expect(render(template, { x: true })).to eq('yes')
      end
    end

    context 'with whitespace control' do
      it 'removes indentation with {[-' do
        template = "line1\n  {[- name ]}"
        expect(render(template, { name: 'Alice' })).to eq("line1\nAlice")
      end

      it 'removes newline with -]}' do
        template = "{[ name -]}\nnext"
        expect(render(template, { name: 'Alice' })).to eq('Alicenext')
      end

      it 'removes both with {[- ... -]}' do
        template = "before\n  {[- name -]}\nafter"
        expect(render(template, { name: 'Alice' })).to eq("before\nAliceafter")
      end

      it 'cleans up control structure lines' do
        template = "<ul>\n  {[-#each items as item-]}\n  <li>{[ item ]}</li>\n  {[-/each-]}\n</ul>"
        expect(render(template, { items: %w[a b] })).to eq("<ul>\n  <li>a</li>\n  <li>b</li>\n</ul>")
      end

      it 'handles if blocks with whitespace control' do
        template = "{[-#if x-]}\nyes\n{[-/if-]}\n"
        expect(render(template, { x: true })).to eq("yes\n")
      end
    end

    context 'with complex templates' do
      it 'renders pagination example' do
        template = <<~TMPL
          {[#each pagination.pages as page]}{[#if page.current]}[{[ page.num ]}]{[#else]}{[ page.num ]}{[/if]}{[/each]}
        TMPL
        data = {
          pagination: {
            pages: [
              { num: 1, current: false },
              { num: 2, current: true },
              { num: 3, current: false }
            ]
          }
        }
        expect(render(template.strip, data)).to eq('1[2]3')
      end
    end
  end
end
