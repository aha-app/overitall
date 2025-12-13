#!/usr/bin/env ruby

require 'time'

# Simulate a monitoring service
puts "Starting system monitor..."
STDOUT.flush

loop do
  # Faster monitoring - every 0.5 to 1.5 seconds
  sleep(rand(500..1500) / 1000.0)

  timestamp = Time.now.iso8601

  cpu = rand(5..95)
  memory = rand(30..85)
  disk = rand(40..75)
  network = rand(10..100)  # MB/s

  # Color code based on resource usage
  cpu_color = cpu > 80 ? "\e[31m" : (cpu > 60 ? "\e[33m" : "\e[32m")
  mem_color = memory > 80 ? "\e[31m" : (memory > 60 ? "\e[33m" : "\e[32m")
  disk_color = disk > 70 ? "\e[33m" : "\e[32m"
  reset = "\e[0m"

  log_line = "#{timestamp} [\e[1;36mMONITOR\e[0m] CPU: #{cpu_color}#{cpu}%#{reset}, Memory: #{mem_color}#{memory}%#{reset}, Disk: #{disk_color}#{disk}%#{reset}, Network: #{network}MB/s"
  puts log_line
  STDOUT.flush

  # Alert on high resource usage
  if cpu > 80 || memory > 80
    alert_line = "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] \e[1;31mWARNING\e[0m: High resource usage detected! (CPU: #{cpu}%, Memory: #{memory}%)"
    puts alert_line
    STDOUT.flush
  end

  # Occasionally report on specific services
  if rand(5) == 0
    services = ["postgresql", "redis", "nginx", "elasticsearch"]
    service = services.sample
    service_status = rand(100) > 10 ? "\e[32mhealthy\e[0m" : "\e[31munhealthy\e[0m"
    service_line = "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Service check: #{service} is #{service_status}"
    puts service_line
    STDOUT.flush
  end
end
