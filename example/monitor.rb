#!/usr/bin/env ruby

require 'time'

# Simulate a monitoring service
puts "Starting system monitor..."
STDOUT.flush

loop do
  sleep(rand(3..6))

  timestamp = Time.now.iso8601

  cpu = rand(5..95)
  memory = rand(30..85)
  disk = rand(40..75)

  log_line = "#{timestamp} [MONITOR] CPU: #{cpu}%, Memory: #{memory}%, Disk: #{disk}%"
  puts log_line
  STDOUT.flush

  # Alert on high resource usage
  if cpu > 80 || memory > 80
    alert_line = "#{Time.now.iso8601} [MONITOR] WARNING: High resource usage detected!"
    puts alert_line
    STDOUT.flush
  end
end
