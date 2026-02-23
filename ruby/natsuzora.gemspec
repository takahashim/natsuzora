# frozen_string_literal: true

require_relative 'lib/natsuzora/version'

Gem::Specification.new do |spec|
  spec.name = 'natsuzora'
  spec.version = Natsuzora::VERSION
  spec.authors = ['Aozora Bunko']
  spec.email = ['info@aozora.gr.jp']

  spec.summary = 'Minimal template language for safe HTML generation'
  spec.description = <<~DESC.gsub("\n", ' ').strip
    Natsuzora is a minimal, display-only template language designed for
    static HTML generation and Rails preview templates.
  DESC
  spec.homepage = 'https://github.com/aozorabunko/natsuzora'
  spec.license = 'MIT'
  spec.required_ruby_version = '>= 3.2.0'

  spec.metadata['homepage_uri'] = spec.homepage
  spec.metadata['source_code_uri'] = spec.homepage
  spec.metadata['changelog_uri'] = "#{spec.homepage}/blob/main/CHANGELOG.md"
  spec.metadata['rubygems_mfa_required'] = 'true'

  spec.files = Dir.chdir(__dir__) do
    `git ls-files -z`.split("\x0").reject do |f|
      (File.expand_path(f) == __FILE__) ||
        f.start_with?(*%w[bin/ test/ spec/ features/ .git .github appveyor Gemfile])
    end
  end
  spec.require_paths = ['lib']

  spec.add_dependency('lexer_kit', '>= 0.5.0')
end
