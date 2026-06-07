#!/usr/bin/env ruby
# frozen_string_literal: true

root = ARGV.fetch(0) do
  warn "usage: #{$PROGRAM_NAME} INSPECT_DIR [RUNNER_OS]"
  exit 2
end
runner_os = ARGV.fetch(1, ENV.fetch("RUNNER_OS", RbConfig::CONFIG["host_os"]))

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
require_match(files, "cefari-desktop", windows ? /cefari-desktop\.exe$/ : /cefari-desktop$/)
require_match(files, "generated frontend", %r{/frontend/index\.html$})
require_match(files, "generated daemon", windows ? /cefari-daemon\.exe$/ : /cefari-daemon$/)
require_match(files, "CEF archive metadata", %r{/cef/archive\.json$})
require_match(files, "CEF payload resources", %r{/cef/(?!archive\.json$).+})
