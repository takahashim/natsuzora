# frozen_string_literal: true

require 'spec_helper'

RSpec.describe 'Include with real files' do
  let(:fixtures_dir) { File.expand_path('fixtures', __dir__) }
  let(:templates_dir) { File.join(fixtures_dir, 'templates') }
  let(:include_root) { File.join(templates_dir, 'shared') }

  def render_template(name, data)
    source = File.read(File.join(templates_dir, "#{name}.tmpl"))
    Natsuzora.render(source, data, include_root: include_root)
  end

  def render_source(source, data)
    Natsuzora.render(source, data, include_root: include_root)
  end

  describe 'basic include' do
    it 'includes a simple partial' do
      result = render_template('simple', {})
      expect(result.strip).to eq('Hello from simple partial')
    end

    it 'includes a partial with argument' do
      result = render_template('greeting', { user: { name: 'Alice' } })
      expect(result.strip).to eq('Hello, Alice!')
    end

    it 'includes a partial directly' do
      result = render_source('{[> /greeting name=name]}', { name: 'Bob' })
      expect(result.strip).to eq('Hello, Bob!')
    end
  end

  describe 'nested path includes' do
    it 'includes from nested directory' do
      result = render_source(
        '{[> /components/card title=t body=b]}',
        { t: 'Title', b: 'Body text' }
      )
      expect(result).to include('<div class="card">')
      expect(result).to include('<h2>Title</h2>')
      expect(result).to include('<p>Body text</p>')
    end

    it 'includes button component' do
      result = render_source(
        '{[> /components/button className=cls label=lbl]}',
        { cls: 'btn-primary', lbl: 'Click me' }
      )
      expect(result.strip).to eq('<button class="btn-primary">Click me</button>')
    end
  end

  describe 'include with each loop' do
    it 'renders multiple cards' do
      result = render_template('card_list', {
        cards: [
          { title: 'Card 1', body: 'Body 1' },
          { title: 'Card 2', body: 'Body 2' }
        ]
      })
      expect(result).to include('<h2>Card 1</h2>')
      expect(result).to include('<h2>Card 2</h2>')
      expect(result).to include('<p>Body 1</p>')
      expect(result).to include('<p>Body 2</p>')
      expect(result.scan('<div class="card">').count).to eq(2)
    end
  end

  describe 'two-level nested includes' do
    # /nav/menu includes /nav/item
    it 'renders navigation menu with items' do
      result = render_template('nav_only', {
        navItems: [
          { label: 'Home', url: '/', active: true },
          { label: 'About', url: '/about', active: false },
          { label: 'Contact', url: '/contact', active: false }
        ]
      })

      expect(result).to include('<nav>')
      expect(result).to include('<ul>')
      expect(result).to include('<strong>Home</strong>')
      expect(result).to include('<a href="/about">About</a>')
      expect(result).to include('<a href="/contact">Contact</a>')
      expect(result.scan('<li>').count).to eq(3)
    end
  end

  describe 'three-level nested includes' do
    # /layout/page includes /layout/header, /layout/footer, /components/card
    # /layout/header includes /nav/menu
    # /nav/menu includes /nav/item
    it 'renders full page layout with deep nesting' do
      result = render_template('full_page', {
        site: {
          title: 'My Site',
          year: 2024,
          nav: [
            { label: 'Home', url: '/', active: true },
            { label: 'Blog', url: '/blog', active: false }
          ]
        },
        page: {
          title: 'Welcome',
          cards: [
            { title: 'Feature 1', body: 'Description 1' },
            { title: 'Feature 2', body: 'Description 2' }
          ]
        }
      })

      # Check HTML structure
      expect(result).to include('<!DOCTYPE html>')
      expect(result).to include('<title>Welcome - My Site</title>')

      # Check header with nested nav
      expect(result).to include('<header>')
      expect(result).to include('<h1>My Site</h1>')
      expect(result).to include('<nav>')
      expect(result).to include('<strong>Home</strong>')
      expect(result).to include('<a href="/blog">Blog</a>')

      # Check main content with cards
      expect(result).to include('<main>')
      expect(result).to include('<h2>Welcome</h2>')
      expect(result).to include('<div class="cards">')
      expect(result).to include('<h2>Feature 1</h2>')
      expect(result).to include('<h2>Feature 2</h2>')

      # Check footer
      expect(result).to include('<footer>')
      expect(result).to include('&copy; 2024 My Site')
    end

    it 'renders page without cards when empty array provided' do
      result = render_template('full_page', {
        site: {
          title: 'My Site',
          year: 2024,
          nav: []
        },
        page: {
          title: 'Empty Page',
          cards: []
        }
      })

      expect(result).to include('<title>Empty Page - My Site</title>')
      expect(result).not_to include('<div class="cards">')
    end
  end

  describe 'include argument shadowing' do
    it 'allows shadowing in include scope' do
      # Create a template that shadows a variable name
      result = render_source(
        '{[ name ]} -> {[> /greeting name=other]} -> {[ name ]}',
        { name: 'Original', other: 'Shadowed' }
      )
      expect(result.strip).to eq("Original -> Hello, Shadowed!\n -> Original")
    end
  end

  describe 'include with path arguments' do
    it 'passes nested path as argument' do
      result = render_source(
        '{[> /greeting name=user.profile.displayName]}',
        { user: { profile: { displayName: 'Charlie' } } }
      )
      expect(result.strip).to eq('Hello, Charlie!')
    end
  end

  describe 'include inside conditional' do
    it 'conditionally includes partial' do
      result = render_source(
        '{[#if showGreeting]}{[> /greeting name=name]}{[/if]}',
        { showGreeting: true, name: 'Dave' }
      )
      expect(result.strip).to eq('Hello, Dave!')

      result = render_source(
        '{[#if showGreeting]}{[> /greeting name=name]}{[/if]}',
        { showGreeting: false, name: 'Dave' }
      )
      expect(result).to eq('')
    end
  end

  describe 'error cases' do
    it 'raises error for missing partial' do
      expect {
        render_source('{[> /nonexistent]}', {})
      }.to raise_error(Natsuzora::IncludeError, /not found/)
    end

    it 'raises error for invalid include name with double dot' do
      # Note: '..' in path causes parse error because '.' is not valid in include names
      expect {
        render_source('{[> /path/../traversal]}', {})
      }.to raise_error(Natsuzora::ParseError)
    end

    it 'raises error for include name with double slash' do
      # Note: '//' causes parse error at lexer level because second '/' is not followed by valid char
      expect {
        render_source('{[> /path//double]}', {})
      }.to raise_error(Natsuzora::ParseError)
    end
  end
end
