#!/usr/bin/env ruby

require 'time'

# Simulate a background worker processing jobs
puts "Starting background worker..."
STDOUT.flush

log_file = File.open("worker.log", "a")

job_count = 0
loop do
  # Much faster - sleep 0.1 to 0.5 seconds (100-500ms)
  sleep(rand(100..500) / 1000.0)

  job_count += 1
  timestamp = Time.now.iso8601

  job_types = ["EmailJob", "ImageProcessingJob", "ReportGenerationJob", "DataSyncJob"]
  job_type = job_types.sample

  log_line = "#{timestamp} [\e[1;35mWORKER\e[0m] \e[36mProcessing\e[0m #{job_type} (job_id: #{job_count})"
  puts log_line
  STDOUT.flush
  log_file.puts log_line
  log_file.flush

  # Faster processing time - 0.05 to 0.3 seconds (50-300ms)
  sleep(rand(50..300) / 1000.0)

  outcome = rand(10)
  if outcome < 7
    complete_line = "#{Time.now.iso8601} [\e[1;35mWORKER\e[0m] \e[1;32mCompleted\e[0m #{job_type} (job_id: #{job_count})"
    puts complete_line
    STDOUT.flush
    log_file.puts complete_line
    log_file.flush
  elsif outcome < 9
    error_line = "#{Time.now.iso8601} [\e[1;35mWORKER\e[0m] \e[1;33mFAILED\e[0m #{job_type} (job_id: #{job_count}) - Retry scheduled"
    puts error_line
    STDOUT.flush
    log_file.puts error_line
    log_file.flush
  else
    # Long stack trace to test line truncation
    stack_trace = "RuntimeError: Failed to process image - ImageMagick error: convert: unable to open image `/tmp/upload_12345.jpg': No such file or directory @ error/blob.c/OpenBlob/2924. at /usr/local/lib/ruby/gems/3.2.0/gems/mini_magick-4.12.0/lib/mini_magick/tool.rb:88:in `run' at /usr/local/lib/ruby/gems/3.2.0/gems/mini_magick-4.12.0/lib/mini_magick/image.rb:234:in `combine_options' at /app/lib/image_processor.rb:45:in `resize_and_optimize' at /app/jobs/image_processing_job.rb:12:in `perform'"
    trace_line = "#{Time.now.iso8601} [\e[1;35mWORKER\e[0m] \e[1;31mEXCEPTION\e[0m in #{job_type}: #{stack_trace}"
    puts trace_line
    STDOUT.flush
    log_file.puts trace_line
    log_file.flush
  end

  # Occasionally log complex job data
  if rand(10) == 0
    job_data = "#{Time.now.iso8601} [WORKER] Job Details: {job_id: #{job_count}, type: '#{job_type}', params: {user_id: #{rand(1000..9999)}, batch_size: #{rand(100..1000)}, options: {priority: 'high', retry_limit: 3, timeout: 300, notification_channels: ['email', 'slack', 'webhook'], metadata: {source: 'api', version: '2.1', region: 'us-east-1'}}}, scheduled_at: '#{Time.now.iso8601}', estimated_duration: #{rand(10..120)} seconds}"
    puts job_data
    STDOUT.flush
    log_file.puts job_data
    log_file.flush
  end
end
