#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "json"
require "optparse"
require "pathname"

ROOT = Pathname.new(__dir__).join("..").expand_path

TARGETS = {
  "darwin-arm64" => {
    package: "@cefari/cli-darwin-arm64",
    binary: "cefari",
    desktop: "cefari-desktop"
  },
  "darwin-x64" => {
    package: "@cefari/cli-darwin-x64",
    binary: "cefari",
    desktop: "cefari-desktop"
  },
  "linux-x64" => {
    package: "@cefari/cli-linux-x64",
    binary: "cefari",
    desktop: "cefari-desktop"
  },
  "win32-x64" => {
    package: "@cefari/cli-win32-x64",
    binary: "cefari.exe",
    desktop: "cefari-desktop.exe"
  }
}.freeze

options = {
  output: ROOT.join(".release/npm").to_s
}

OptionParser.new do |parser|
  parser.banner = "Usage: assemble-npm-cli-packages.rb [options]"
  parser.on("--version VERSION", "Package version. Defaults to crates/cefari-cli/Cargo.toml.") do |value|
    options[:version] = value
  end
  parser.on("--output DIR", "Output directory for assembled packages.") do |value|
    options[:output] = value
  end
  parser.on("--root-only", "Assemble only the @cefari/cli package.") do
    options[:root_only] = true
  end
  parser.on("--target TARGET", "Platform target: #{TARGETS.keys.join(", ")}") do |value|
    options[:target] = value
  end
  parser.on("--cli-binary PATH", "Path to the built cefari binary.") do |value|
    options[:cli_binary] = value
  end
  parser.on("--desktop-binary PATH", "Path to the built cefari-desktop runtime.") do |value|
    options[:desktop_binary] = value
  end
end.parse!

def package_version
  manifest = ROOT.join("crates/cefari-cli/Cargo.toml").read
  match = manifest.match(/^version\s*=\s*"([^"]+)"/)
  abort "failed to read cefari-cli version" unless match

  match[1]
end

def package_dir_name(package_name)
  package_name.sub("@", "").tr("/", "-")
end

def write_json(path, data)
  path.write("#{JSON.pretty_generate(data)}\n")
end

def copy_template(source, destination)
  FileUtils.rm_rf(destination)
  FileUtils.mkdir_p(destination.dirname)
  FileUtils.cp_r(source, destination)
end

def assemble_root(output, version)
  source = ROOT.join("npm/cli")
  destination = output.join("packages/cefari-cli")
  copy_template(source, destination)

  manifest_path = destination.join("package.json")
  manifest = JSON.parse(manifest_path.read)
  manifest["version"] = version
  manifest.delete("private")
  manifest["optionalDependencies"] = TARGETS.transform_values { |_target| version }
  write_json(manifest_path, manifest)
  FileUtils.chmod(0o755, destination.join("bin/cefari.js"))
  destination
end

def assemble_platform(output, target, cli_binary, desktop_binary, version)
  config = TARGETS.fetch(target)
  source = ROOT.join("npm/platform/#{target}")
  abort "missing npm platform template for #{target}" unless source.directory?
  abort "missing cefari binary: #{cli_binary}" unless File.file?(cli_binary)
  abort "missing cefari-desktop runtime: #{desktop_binary}" unless File.file?(desktop_binary)

  destination = output.join("packages/#{package_dir_name(config.fetch(:package))}")
  copy_template(source, destination)

  manifest_path = destination.join("package.json")
  manifest = JSON.parse(manifest_path.read)
  manifest["version"] = version
  manifest.delete("private")
  write_json(manifest_path, manifest)

  bin_dir = destination.join("bin")
  runtime_dir = destination.join("libexec/cefari")
  FileUtils.mkdir_p(bin_dir)
  FileUtils.mkdir_p(runtime_dir)
  FileUtils.cp(cli_binary, bin_dir.join(config.fetch(:binary)))
  FileUtils.cp(desktop_binary, runtime_dir.join(config.fetch(:desktop)))
  FileUtils.chmod(0o755, bin_dir.join(config.fetch(:binary))) unless target.start_with?("win32-")
  FileUtils.chmod(0o755, runtime_dir.join(config.fetch(:desktop))) unless target.start_with?("win32-")
  destination
end

version = options[:version] || package_version
output = Pathname.new(options.fetch(:output)).expand_path
FileUtils.mkdir_p(output)

if options[:root_only]
  puts assemble_root(output, version)
  exit
end

target = options[:target]
abort "missing --target" unless target
abort "unsupported target #{target}; expected one of #{TARGETS.keys.join(", ")}" unless TARGETS.key?(target)
abort "missing --cli-binary" unless options[:cli_binary]
abort "missing --desktop-binary" unless options[:desktop_binary]

puts assemble_platform(
  output,
  target,
  Pathname.new(options.fetch(:cli_binary)).expand_path,
  Pathname.new(options.fetch(:desktop_binary)).expand_path,
  version
)
