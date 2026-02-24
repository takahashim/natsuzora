# frozen_string_literal: true

# Renderer層のエラーメッセージ内容を検証するテスト。
# エラーの発生自体はJSON共有テストスイート(errors.json等)でカバー済み。
# ここではメッセージ文字列の退行を防ぐ。
RSpec.describe Natsuzora do
  def render(source, data)
    described_class.render(source, data)
  end

  describe 'TypeError messages' do
    it 'includes "null" for null without modifier' do
      expect { render('{[ x ]}', { x: nil }) }
        .to raise_error(Natsuzora::TypeError, /null/)
    end

    it 'includes "boolean" for boolean stringification' do
      expect { render('{[ x ]}', { x: true }) }
        .to raise_error(Natsuzora::TypeError, /boolean/)
    end

    it 'includes "array" for array stringification' do
      expect { render('{[ x ]}', { x: [1] }) }
        .to raise_error(Natsuzora::TypeError, /array/)
    end

    it 'includes "object" for object stringification' do
      expect { render('{[ x ]}', { x: { a: 1 } }) }
        .to raise_error(Natsuzora::TypeError, /object/)
    end

    it 'includes "null" for required modifier with null' do
      expect { render('{[ x! ]}', { x: nil }) }
        .to raise_error(Natsuzora::TypeError, /null/)
    end

    it 'includes "empty" for required modifier with empty string' do
      expect { render('{[ x! ]}', { x: '' }) }
        .to raise_error(Natsuzora::TypeError, /empty/)
    end

    it 'includes "array" for each on non-array' do
      expect { render('{[#each x as i]}{[/each]}', { x: 'str' }) }
        .to raise_error(Natsuzora::TypeError, /array/)
    end
  end

  describe 'UndefinedVariableError' do
    it 'includes variable name for undefined variable' do
      expect { render('{[ unknown ]}', {}) }
        .to raise_error(Natsuzora::UndefinedVariableError, /unknown/)
    end

    it 'includes property name for undefined nested path' do
      expect { render('{[ user.missing ]}', { user: {} }) }
        .to raise_error(Natsuzora::UndefinedVariableError, /missing/)
    end
  end

  describe 'ShadowingError' do
    it 'includes variable name when shadowing root variable' do
      expect { render('{[#each items as name]}{[/each]}', { items: [1], name: 'x' }) }
        .to raise_error(Natsuzora::ShadowingError, /name/)
    end

    it 'includes variable name when shadowing outer each variable' do
      expect { render('{[#each a as x]}{[#each b as x]}{[/each]}{[/each]}', { a: [1], b: [1] }) }
        .to raise_error(Natsuzora::ShadowingError, /x/)
    end
  end
end
