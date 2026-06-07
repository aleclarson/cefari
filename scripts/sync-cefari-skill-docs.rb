#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"

ROOT = File.expand_path("..", __dir__)
SOURCE_DIR = File.join(ROOT, "docs")
TARGET_DIR = File.join(ROOT, "skills", "cefari", "docs")
EXCLUDED_FILES = [
  "architecture.md",
  "ipc.md",
  "runtime/notifications.md",
  "typescript/raw-ipc.md",
  "verification.md",
].freeze

def relative_files(root)
  Dir.glob(File.join(root, "**", "*"), File::FNM_DOTMATCH)
    .select { |path| File.file?(path) }
    .map { |path| path.delete_prefix("#{root}/") }
    .reject { |path| EXCLUDED_FILES.include?(path) }
    .sort
end

def check_sync
  unless Dir.exist?(TARGET_DIR)
    warn "#{TARGET_DIR.delete_prefix("#{ROOT}/")} does not exist. Run scripts/sync-cefari-skill-docs.rb."
    return false
  end

  source_files = relative_files(SOURCE_DIR)
  target_files = relative_files(TARGET_DIR)
  ok = true

  extra_source = source_files - target_files
  extra_target = target_files - source_files
  changed = (source_files & target_files).reject do |path|
    File.binread(File.join(SOURCE_DIR, path)) == File.binread(File.join(TARGET_DIR, path))
  end

  unless extra_source.empty?
    warn "Missing files in skills/cefari/docs:"
    extra_source.each { |path| warn "  #{path}" }
    ok = false
  end

  unless extra_target.empty?
    warn "Extra files in skills/cefari/docs:"
    extra_target.each { |path| warn "  #{path}" }
    ok = false
  end

  unless changed.empty?
    warn "Changed files in skills/cefari/docs:"
    changed.each { |path| warn "  #{path}" }
    ok = false
  end

  warn "skills/cefari/docs is stale. Run scripts/sync-cefari-skill-docs.rb." unless ok
  ok
end

case ARGV
when ["--check"]
  exit(check_sync ? 0 : 1)
when []
  FileUtils.rm_rf(TARGET_DIR)
  FileUtils.mkdir_p(TARGET_DIR)
  relative_files(SOURCE_DIR).each do |path|
    target = File.join(TARGET_DIR, path)
    FileUtils.mkdir_p(File.dirname(target))
    FileUtils.cp(File.join(SOURCE_DIR, path), target)
  end
else
  warn "usage: #{$PROGRAM_NAME} [--check]"
  exit 2
end
