#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"

root = ARGV.fetch(0) do
  warn "usage: #{$PROGRAM_NAME} INSPECT_DIR [RUNNER_OS] [MANIFEST_JSON]"
  exit 2
end
runner_os = ARGV.fetch(1, ENV.fetch("RUNNER_OS", RbConfig::CONFIG["host_os"]))
manifest_path = ARGV[2]
manifest = manifest_path ? JSON.parse(File.read(manifest_path)) : nil

files = Dir.glob("#{root}/**/*", File::FNM_DOTMATCH)
  .select { |path| File.file?(path) }
  .map { |path| path.tr("\\", "/") }

def require_match(files, description, pattern)
  return if files.any? { |path| path.match?(pattern) }

  warn "inspected files:"
  files.each { |path| warn "  #{path}" }
  abort "native package payload is missing #{description}"
end

windows = runner_os.match?(/windows/i)
if manifest_path
  desktop_binary = manifest.fetch("desktop_binary")
  require_match(files, desktop_binary, /#{Regexp.escape(desktop_binary)}$/)
else
  require_match(files, "cefari-desktop", windows ? /cefari-desktop\.exe$/ : /cefari-desktop$/)
end
require_match(files, "generated frontend", %r{/frontend/index\.html$})
if manifest
  daemon_executable = File.basename(manifest.fetch("daemon_executable"))
  require_match(files, daemon_executable, /#{Regexp.escape(daemon_executable)}$/)
else
  require_match(files, "generated daemon", windows ? /cefari-daemon\.exe$/ : /cefari-daemon$/)
end
require_match(files, "CEF archive metadata", %r{/cef/archive\.json$})
require_match(files, "CEF payload resources", %r{/cef/(?!archive\.json$).+})
