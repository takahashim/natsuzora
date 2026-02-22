#!/usr/bin/env ruby
# frozen_string_literal: true

require 'json'
require 'fileutils'
require_relative '../../ruby/lib/natsuzora'

BLOG_ROOT = File.expand_path(__dir__)
SHARED_DIR = File.join(BLOG_ROOT, 'shared')
DIST_DIR = File.join(BLOG_ROOT, 'dist')

def render_page(template_path, data_path, output_path)
  source = File.read(template_path)
  data = JSON.parse(File.read(data_path))
  template = Natsuzora::Template.new(source, include_root: SHARED_DIR)
  html = template.render(data)

  FileUtils.mkdir_p(File.dirname(output_path))
  File.write(output_path, html)
rescue StandardError => e
  context = "template=#{template_path}, data=#{data_path}, output=#{output_path}"
  message = "#{e.message}\n  while rendering #{context}"
  raise e.class, message
end

FileUtils.rm_rf(DIST_DIR)

pages = [
  { template: 'index.ntzr', data: 'data.json', output: 'index.html' },
  { template: 'profile.ntzr', data: 'data.json', output: 'profile/index.html' },
  {
    template: 'post.ntzr',
    data: 'post-component-design-best-practices.json',
    output: 'posts/component-design-best-practices/index.html'
  },
  {
    template: 'post.ntzr',
    data: 'post-include-safety-checklist.json',
    output: 'posts/include-safety-checklist/index.html'
  },
  {
    template: 'category.ntzr',
    data: 'category-engineering.json',
    output: 'categories/engineering/index.html'
  }
]

pages.each do |page|
  render_page(
    File.join(BLOG_ROOT, page[:template]),
    File.join(BLOG_ROOT, page[:data]),
    File.join(DIST_DIR, page[:output])
  )
end

puts "Blog sample rendered to #{DIST_DIR}"
