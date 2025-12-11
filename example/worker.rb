#!/usr/bin/env ruby

require 'time'

# Simulate a background worker processing jobs
puts "Starting background worker..."
STDOUT.flush

log_file = File.open("example/worker.log", "a")

job_count = 0
loop do
  sleep(rand(2..5))

  job_count += 1
  timestamp = Time.now.iso8601

  job_types = ["EmailJob", "ImageProcessingJob", "ReportGenerationJob", "DataSyncJob"]
  job_type = job_types.sample

  log_line = "#{timestamp} [WORKER] Processing #{job_type} (job_id: #{job_count})"
  puts log_line
  STDOUT.flush
  log_file.puts log_line
  log_file.flush

  sleep(rand(1..2))

  if rand(10) < 8
    complete_line = "#{Time.now.iso8601} [WORKER] Completed #{job_type} (job_id: #{job_count})"
    puts complete_line
    STDOUT.flush
    log_file.puts complete_line
    log_file.flush
  else
    error_line = "#{Time.now.iso8601} [WORKER] FAILED #{job_type} (job_id: #{job_count}) - Retry scheduled"
    puts error_line
    STDOUT.flush
    log_file.puts error_line
    log_file.flush
  end
end
