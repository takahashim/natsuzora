# frozen_string_literal: true

RSpec.describe Natsuzora::Parser do
  def parse(source)
    tokens = Natsuzora::Lexer.new(source).tokenize
    described_class.new(tokens).parse
  end

  describe '#parse' do
    context 'with text' do
      it 'parses plain text' do
        ast = parse('Hello World')
        expect(ast.nodes.first).to be_a(Natsuzora::AST::Text)
        expect(ast.nodes.first.content).to eq('Hello World')
      end
    end

    context 'with variables' do
      it 'parses simple variable' do
        ast = parse('{[ name ]}')
        expect(ast.nodes.first).to be_a(Natsuzora::AST::Variable)
        expect(ast.nodes.first.path).to eq(['name'])
      end

      it 'parses nested path' do
        ast = parse('{[ user.profile.name ]}')
        expect(ast.nodes.first.path).to eq(%w[user profile name])
      end
    end

    context 'with if blocks' do
      it 'parses if without else' do
        ast = parse('{[#if visible]}content{[/if]}')
        node = ast.nodes.first
        expect(node).to be_a(Natsuzora::AST::IfBlock)
        expect(node.condition.path).to eq(['visible'])
        expect(node.then_nodes).not_to be_empty
        expect(node.else_nodes).to be_nil
      end

      it 'parses if with else' do
        ast = parse('{[#if visible]}yes{[#else]}no{[/if]}')
        node = ast.nodes.first
        expect(node.then_nodes.first.content).to eq('yes')
        expect(node.else_nodes.first.content).to eq('no')
      end

      it 'parses nested if blocks' do
        ast = parse('{[#if a]}{[#if b]}inner{[/if]}{[/if]}')
        outer = ast.nodes.first
        inner = outer.then_nodes.first
        expect(inner).to be_a(Natsuzora::AST::IfBlock)
      end
    end

    context 'with unless blocks' do
      it 'parses unless block' do
        ast = parse('{[#unless hidden]}content{[/unless]}')
        node = ast.nodes.first
        expect(node).to be_a(Natsuzora::AST::UnlessBlock)
        expect(node.condition.path).to eq(['hidden'])
        expect(node.body_nodes).not_to be_empty
      end

      it 'parses nested unless blocks' do
        ast = parse('{[#unless a]}{[#unless b]}inner{[/unless]}{[/unless]}')
        outer = ast.nodes.first
        inner = outer.body_nodes.first
        expect(inner).to be_a(Natsuzora::AST::UnlessBlock)
      end
    end

    context 'with each blocks' do
      it 'parses each block' do
        ast = parse('{[#each items as item]}{[ item ]}{[/each]}')
        node = ast.nodes.first
        expect(node).to be_a(Natsuzora::AST::EachBlock)
        expect(node.collection.path).to eq(['items'])
        expect(node.item_name).to eq('item')
      end
    end

    context 'with unsecure output' do
      it 'parses unsecure output' do
        ast = parse('{[!unsecure html ]}')
        node = ast.nodes.first
        expect(node).to be_a(Natsuzora::AST::UnsecureOutput)
        expect(node.path.path).to eq(['html'])
      end
    end

    context 'with include' do
      it 'parses include without arguments' do
        ast = parse('{[!include /card ]}')
        node = ast.nodes.first
        expect(node).to be_a(Natsuzora::AST::Include)
        expect(node.name).to eq('/card')
        expect(node.args).to be_empty
      end

      it 'parses include with arguments' do
        ast = parse('{[!include /card title=heading ]}')
        node = ast.nodes.first
        expect(node.args.keys).to eq(['title'])
        expect(node.args['title'].path).to eq(['heading'])
      end

      it 'parses include with multiple arguments' do
        ast = parse('{[!include /card title=a body=b ]}')
        node = ast.nodes.first
        expect(node.args.keys).to contain_exactly('title', 'body')
      end
    end

    context 'with reserved words' do
      it 'rejects reserved word as variable' do
        expect { parse('{[ if ]}') }.to raise_error(Natsuzora::ReservedWordError, /'if'/)
      end

      it 'rejects reserved word as each item' do
        expect { parse('{[#each items as true]}{[/each]}') }
          .to raise_error(Natsuzora::ReservedWordError, /'true'/)
      end

      %w[if unless each as unsecure true false null include].each do |word|
        it "rejects '#{word}' as identifier" do
          expect { parse("{[ #{word} ]}") }.to raise_error(Natsuzora::ReservedWordError)
        end
      end
    end

    context 'with invalid identifiers' do
      it 'rejects underscore prefix at lexer level' do
        expect { parse('{[ _private ]}') }.to raise_error(Natsuzora::LexerError, /Unexpected character/)
      end

      it 'rejects @ in identifier at lexer level' do
        expect { parse('{[ foo@bar ]}') }.to raise_error(Natsuzora::LexerError, /Unexpected character/)
      end
    end

    context 'with invalid include names' do
      it 'rejects include name without leading slash' do
        expect { parse('{[!include card ]}') }.to raise_error(Natsuzora::ParseError, /must start with/)
      end

      it 'rejects include name with ..' do
        # Lexer stops at '.' so this results in a parse error
        expect { parse('{[!include /../card ]}') }.to raise_error(Natsuzora::ParseError)
      end

      it 'rejects include name with // at start' do
        # {[!include //card ]} - first / is parsed, then /card is parsed as slash + ident
        expect { parse('{[!include //card ]}') }.to raise_error(Natsuzora::ParseError)
      end

      it 'rejects include name containing //' do
        # Lexer stops at // so the second / becomes a separate token, causing parse error
        expect { parse('{[!include /a//b ]}') }.to raise_error(Natsuzora::ParseError)
      end

      it 'rejects duplicate include arguments' do
        expect { parse('{[!include /card a=x a=y ]}') }.to raise_error(Natsuzora::ParseError, /Duplicate/)
      end
    end

    context 'with syntax errors' do
      it 'raises error for unclosed block' do
        expect { parse('{[#if x]}') }.to raise_error(Natsuzora::ParseError)
      end

      it 'raises error for mismatched block close' do
        expect { parse('{[#if x]}{[/each]}') }.to raise_error(Natsuzora::ParseError)
      end

      it 'raises error for standalone else' do
        expect { parse('{[#else]}') }.to raise_error(Natsuzora::ParseError)
      end
    end
  end
end
