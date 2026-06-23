#!/usr/bin/env ruby

require 'rbconfig'
require 'time'

STDOUT.sync = true

SCRIPT_PATH = File.expand_path(__FILE__)

# Spawn child Ruby processes from this same file so the process tree has
# stable, readable command names without extra example files.
def spawn_monitor_child(*args)
  Process.spawn(RbConfig.ruby, SCRIPT_PATH, *args)
end

def terminate_children(pids)
  children = pids.compact.uniq
  children.each do |pid|
    Process.kill('TERM', pid)
  rescue Errno::ESRCH
  end

  deadline = Time.now + 2
  remaining = children.dup
  until remaining.empty? || Time.now >= deadline
    remaining.reject! do |pid|
      begin
        Process.wait(pid, Process::WNOHANG)
      rescue Errno::ECHILD
        true
      end
    end
    sleep 0.05 unless remaining.empty?
  end

  remaining.each do |pid|
    Process.kill('KILL', pid)
  rescue Errno::ESRCH
  end
  remaining.each do |pid|
    Process.wait(pid)
  rescue Errno::ECHILD
  end
end

def install_shutdown_traps(pids)
  ['INT', 'TERM'].each do |signal|
    trap(signal) do
      terminate_children(pids)
      exit
    end
  end
end

def run_probe(name)
  puts "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Probe #{name} started (pid: #{Process.pid}, ppid: #{Process.ppid})"
  loop do
    sleep 10
    puts "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Probe #{name} heartbeat (pid: #{Process.pid})"
  end
end

def run_collector(name)
  probe_pid = spawn_monitor_child('probe', name)
  child_pids = [probe_pid]
  install_shutdown_traps(child_pids)

  puts "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Collector #{name} started (pid: #{Process.pid}, probe_pid: #{probe_pid})"
  loop do
    sleep(rand(2500..4500) / 1000.0)
    puts "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Collector #{name} sampled #{rand(10..99)} units (pid: #{Process.pid})"
  end
ensure
  terminate_children(child_pids || [])
end

def run_supervisor
  child_pids = %w[cpu memory network].map do |collector|
    spawn_monitor_child('collector', collector)
  end
  install_shutdown_traps(child_pids)

  puts "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Supervisor started (pid: #{Process.pid}, collectors: #{child_pids.join(',')})"
  loop do
    sleep 5
    puts "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Supervisor heartbeat (collectors: #{child_pids.join(',')})"
  end
ensure
  terminate_children(child_pids || [])
end

case ARGV[0]
when 'supervisor'
  run_supervisor
when 'collector'
  run_collector(ARGV[1] || 'unknown')
when 'probe'
  run_probe(ARGV[1] || 'unknown')
else
  # Simulate a monitoring service and keep a nested helper process tree alive.
  puts 'Starting system monitor...'
  supervisor_pid = spawn_monitor_child('supervisor')
  child_pids = [supervisor_pid]
  install_shutdown_traps(child_pids)
  puts "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Spawned helper process tree (supervisor_pid: #{supervisor_pid})"

  begin
    loop do
      # Faster monitoring - every 0.5 to 1.5 seconds
      sleep(rand(500..1500) / 1000.0)

      timestamp = Time.now.iso8601

      cpu = rand(5..95)
      memory = rand(30..85)
      disk = rand(40..75)
      network = rand(10..100) # MB/s

      # Color code based on resource usage
      cpu_color = cpu > 80 ? "\e[31m" : (cpu > 60 ? "\e[33m" : "\e[32m")
      mem_color = memory > 80 ? "\e[31m" : (memory > 60 ? "\e[33m" : "\e[32m")
      disk_color = disk > 70 ? "\e[33m" : "\e[32m"
      reset = "\e[0m"

      log_line = "#{timestamp} [\e[1;36mMONITOR\e[0m] CPU: #{cpu_color}#{cpu}%#{reset}, Memory: #{mem_color}#{memory}%#{reset}, Disk: #{disk_color}#{disk}%#{reset}, Network: #{network}MB/s"
      puts log_line

      # Alert on high resource usage
      if cpu > 80 || memory > 80
        alert_line = "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] \e[1;31mWARNING\e[0m: High resource usage detected! (CPU: #{cpu}%, Memory: #{memory}%)"
        puts alert_line
      end

      # Occasionally report on specific services
      if rand(5) == 0
        services = ["postgresql", "redis", "nginx", "elasticsearch"]
        service = services.sample
        service_status = rand(100) > 10 ? "\e[32mhealthy\e[0m" : "\e[31munhealthy\e[0m"
        service_line = "#{Time.now.iso8601} [\e[1;36mMONITOR\e[0m] Service check: #{service} is #{service_status}"
        puts service_line
      end
    end
  ensure
    terminate_children(child_pids || [])
  end
end
