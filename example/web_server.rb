#!/usr/bin/env ruby

require 'time'

# Simulate a web server generating logs
puts "Starting web server on port 3000..."
STDOUT.flush

log_file = File.open("example/web.log", "a")

request_count = 0
loop do
  sleep(rand(1..3))

  request_count += 1
  timestamp = Time.now.iso8601

  paths = ["/", "/api/users", "/api/posts", "/health", "/metrics"]
  methods = ["GET", "POST", "PUT", "DELETE"]
  status_codes = [200, 200, 200, 201, 304, 400, 404, 500]

  method = methods.sample
  path = paths.sample
  status = status_codes.sample
  duration = rand(10..500)

  log_line = "#{timestamp} [WEB] #{method} #{path} - #{status} (#{duration}ms)"

  puts log_line
  STDOUT.flush

  log_file.puts log_line
  log_file.flush

  # Occasionally log errors
  if rand(10) == 0
    error_line = "#{Time.now.iso8601} [WEB] ERROR: Database connection timeout"
    puts error_line
    STDOUT.flush
    log_file.puts error_line
    log_file.flush
  end
end
