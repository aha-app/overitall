#!/usr/bin/env ruby

require 'time'

# Simulate a web server generating logs
puts "Starting web server on port 3000..."
STDOUT.flush

log_file = File.open("web.log", "a")

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

  # Color code based on status
  status_color = case status
  when 200..299 then "\e[32m"  # Green for success
  when 300..399 then "\e[36m"  # Cyan for redirects
  when 400..499 then "\e[33m"  # Yellow for client errors
  when 500..599 then "\e[31m"  # Red for server errors
  else "\e[0m"
  end
  reset = "\e[0m"

  # Color code duration based on speed
  duration_color = if duration < 100
    "\e[32m"  # Green for fast
  elsif duration < 300
    "\e[33m"  # Yellow for medium
  else
    "\e[31m"  # Red for slow
  end

  log_line = "#{timestamp} [\e[1;34mWEB\e[0m] \e[1m#{method}\e[0m #{path} - #{status_color}#{status}#{reset} (#{duration_color}#{duration}ms#{reset})"

  puts log_line
  STDOUT.flush

  log_file.puts log_line
  log_file.flush

  # Occasionally log errors or long SQL queries
  rand_event = rand(10)
  if rand_event == 0
    error_line = "#{Time.now.iso8601} [\e[1;34mWEB\e[0m] \e[1;31mERROR\e[0m: Database connection timeout"
    puts error_line
    STDOUT.flush
    log_file.puts error_line
    log_file.flush
  elsif rand_event == 1
    # Long SQL query to test line truncation
    sql_query = "SELECT users.id, users.name, users.email, users.created_at, users.updated_at, orders.id AS order_id, orders.total, orders.status, orders.created_at AS order_date, products.name AS product_name, products.price, products.category, order_items.quantity, order_items.price AS item_price FROM users INNER JOIN orders ON users.id = orders.user_id INNER JOIN order_items ON orders.id = order_items.order_id INNER JOIN products ON order_items.product_id = products.id WHERE users.active = true AND orders.status IN ('pending', 'processing', 'shipped') AND orders.created_at > '2024-01-01' ORDER BY orders.created_at DESC, users.name ASC LIMIT 100"
    sql_line = "#{Time.now.iso8601} [WEB] SQL QUERY: #{sql_query}"
    puts sql_line
    STDOUT.flush
    log_file.puts sql_line
    log_file.flush
  elsif rand_event == 2
    # Long JSON response
    json_data = '{"status":"success","data":{"users":[{"id":1,"name":"Alice Johnson","email":"alice@example.com","role":"admin","permissions":["read","write","delete"],"metadata":{"last_login":"2024-12-10T10:30:00Z","ip_address":"192.168.1.100"}},{"id":2,"name":"Bob Smith","email":"bob@example.com","role":"user","permissions":["read"],"metadata":{"last_login":"2024-12-09T15:45:00Z","ip_address":"192.168.1.101"}}],"pagination":{"current_page":1,"total_pages":50,"total_count":1000}}}'
    json_line = "#{Time.now.iso8601} [WEB] API RESPONSE: #{json_data}"
    puts json_line
    STDOUT.flush
    log_file.puts json_line
    log_file.flush
  end
end
